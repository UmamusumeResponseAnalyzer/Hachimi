use std::ops::Not;

use crate::{core::{template, Hachimi}, il2cpp::{ext::{Il2CppStringExt, StringExt}, symbols::get_method_addr, types::*}};

type PopulateWithErrorsFn = extern "C" fn(
    this: *mut Il2CppObject, str: *mut Il2CppString,
    settings: TextGenerationSettings_t, context: *mut Il2CppObject
) -> bool;
extern "C" fn PopulateWithErrors(
    this: *mut Il2CppObject, str_: *mut Il2CppString,
    mut settings: TextGenerationSettings_t, context: *mut Il2CppObject
) -> bool {
    let orig_fn = get_orig_fn!(PopulateWithErrors, PopulateWithErrorsFn);
    let localized_data = &Hachimi::instance().localized_data.load();
    let hashed_dict = &localized_data.hashed_dict;

    if let Some(text) = hashed_dict.is_empty().not()
        .then(|| hashed_dict.get(&unsafe { (*str_).hash() }))
        .flatten()
    {
        orig_fn(this, text.to_il2cpp_string(), settings, context)
    }
    else if !localized_data.localize_dict.is_empty() || !localized_data.text_data_dict.is_empty() {
        let str = unsafe { (*str_).as_utf16str() };

        // Only try to evaluate a template if it looks like one
        let new_str = if str.as_slice().contains(&36) { // 36 = dollar sign ($)
            let mut context = TemplateContext {
                settings: &mut settings
            };
            Hachimi::instance().template_parser
                .eval_with_context(&str.to_string(), &mut context)
                .to_il2cpp_string()
        }
        else {
            str_
        };
        orig_fn(this, new_str, settings, context)
    }
    else {
        orig_fn(this, str_, settings, context)
    }
}

struct TemplateContext<'a> {
    settings: &'a mut TextGenerationSettings_t
}

impl<'a> template::Context for TemplateContext<'a> {
    fn on_filter_eval(&mut self, name: &str, args: &[template::Token]) -> Option<String> {
        // Extra filters to modify the text generation settings
        match name {
            "nb" => {
                self.settings.horizontalOverflow = HorizontalWrapMode_Overflow;
                self.settings.generateOutOfBounds = true;
            }
            
            "anchor" => {
                // Anchor values:
                // 1  2  3
                // 4  5  6
                // 7  8  9
                // Example: $(anchor 6) = middle right
                let value = args.get(0)?;
                let template::Token::NumberLit(anchor_num) = *value else {
                    return None;
                };
                let anchor = (anchor_num as i32) - 1;
                if anchor < 0 || anchor > 8 {
                    return None;
                }
                self.settings.textAnchor = anchor;
            }

            "scale" => {
                // Example: $(scale 80) = scale font size to 80%
                let value = args.get(0)?;
                let template::Token::NumberLit(percentage) = value else {
                    return None;
                };
                self.settings.fontSize = (self.settings.fontSize as f64 * (percentage / 100.0)) as i32;
            }

            "ho" => {
                // $(ho 0) or $(ho 1)
                let value = args.get(0)?;
                let template::Token::NumberLit(overflow_num) = *value else {
                    return None;
                };
                let overflow = overflow_num as i32;
                if overflow != 0 && overflow != 1 {
                    return None;
                }
                self.settings.horizontalOverflow = overflow;
            }

            "vo" => {
                // $(vo 0) or $(vo 1)
                let value = args.get(0)?;
                let template::Token::NumberLit(overflow_num) = *value else {
                    return None;
                };
                let overflow = overflow_num as i32;
                if overflow != 0 && overflow != 1 {
                    return None;
                }
                self.settings.verticalOverflow = overflow;
            }

            _ => return None
        }

        Some(String::new())
    }
}

// Context that ignores TextGenerator filters
pub struct IgnoreTGFiltersContext();

impl template::Context for IgnoreTGFiltersContext {
    fn on_filter_eval(&mut self, _name: &str, _args: &[template::Token]) -> Option<String> {
        match _name {
            "nb" | "anchor" | "scale" | "ho" | "vo" => Some(String::new()),
            _ => None
        }
    }
}

pub fn init(UnityEngine_TextRenderingModule: *const Il2CppImage) {
    get_class_or_return!(UnityEngine_TextRenderingModule, UnityEngine, TextGenerator);

    let PopulateWithErrors_addr = get_method_addr(TextGenerator, c"PopulateWithErrors", 3);

    new_hook!(PopulateWithErrors_addr, PopulateWithErrors);
}