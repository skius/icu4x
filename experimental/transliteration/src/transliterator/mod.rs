// This file is part of ICU4X. For terms of use, please see the file
// called LICENSE at the top level of the ICU4X source tree
// (online at: https://github.com/unicode-org/icu4x/blob/main/LICENSE ).

mod borrowed;
mod replaceable;

use crate::provider::{RuleBasedTransliterator, TransliteratorRulesV1Marker};
use crate::TransliteratorError;
use alloc::borrow::Cow;
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use icu_collections::codepointinvlist::CodePointInversionList;
use icu_provider::_internal::locid::Locale;
use icu_provider::{DataError, DataLocale, DataPayload, DataProvider, DataRequest};

use borrowed::*;
use litemap::LiteMap;
use replaceable::*;

type Filter<'a> = CodePointInversionList<'a>;
// non-alloc way of taking the intersection of multiple filters
#[derive(Debug)]
pub struct FilterChain<'a>(Option<(&'a Filter<'a>, &'a FilterChain<'a>)>);

impl<'a> FilterChain<'a> {
    fn new() -> Self {
        Self(None)
    }

    fn add(&'a self, filter: &'a Filter<'a>) -> Self {
        Self(Some((filter, self)))
    }

    fn contains(&self, c: char) -> bool {
        match self.0 {
            None => false,
            Some((set, next)) => set.contains(c) && next.contains(c),
        }
    }
}

pub trait CustomTransliterator {
    fn transliterate(&self, input: &str, filter: &FilterChain) -> String;
}

struct NFCTransliterator {}

enum InternalTransliterator {
    RuleBased(DataPayload<TransliteratorRulesV1Marker>),
    NFC(NFCTransliterator),
    Dyn(Box<dyn CustomTransliterator>),
}

struct Transliterator {
    transliterator: DataPayload<TransliteratorRulesV1Marker>,
    env: LiteMap<String, InternalTransliterator>,
}

impl Transliterator {
    #[cfg(feature = "compiled_data")]
    pub fn try_new(locale: Locale) -> Result<Transliterator, TransliteratorError> {
        let provider = crate::provider::Baked;
        Self::try_new_unstable(locale, &provider)
    }

    pub fn try_new_unstable<P>(
        locale: Locale,
        provider: &P,
    ) -> Result<Transliterator, TransliteratorError>
    where
        P: DataProvider<TransliteratorRulesV1Marker>,
    {
        debug_assert!(!locale.extensions.transform.is_empty());

        let mut data_locale = DataLocale::default();
        data_locale.set_aux(locale.to_string().parse()?);
        let req = DataRequest {
            locale: &data_locale,
            metadata: Default::default(),
        };
        let rbt = provider.load(req)?.take_payload()?;
        let mut env = LiteMap::new();
        Transliterator::load_dependencies(rbt.get(), &mut env, provider)?;
        Ok(Transliterator {
            transliterator: rbt,
            env,
        })
    }

    fn load_dependencies<P>(
        rbt: &RuleBasedTransliterator<'_>,
        env: &mut LiteMap<String, InternalTransliterator>,
        provider: &P,
    ) -> Result<(), TransliteratorError>
    where
        P: DataProvider<TransliteratorRulesV1Marker>,
    {
        for dep in rbt.dependencies.iter() {
            if !env.contains_key(dep) {
                let internal_t = Self::load_nested(dep, provider)?;
                if let InternalTransliterator::RuleBased(rbt) = &internal_t {
                    Self::load_dependencies(rbt.get(), env, provider)?;
                }
                env.insert(dep.to_string(), internal_t);
            }
        }
        Ok(())
    }

    // TODO: add hook for custom
    fn load_nested<P>(id: &str, provider: &P) -> Result<InternalTransliterator, TransliteratorError>
    where
        P: DataProvider<TransliteratorRulesV1Marker>,
    {
        if let Some(special) = id.strip_prefix("x-") {
            match special {
                "Any-NFC" => Ok(InternalTransliterator::NFC(NFCTransliterator {})),
                _ => Ok(InternalTransliterator::NFC(NFCTransliterator {})),
                s => Err(DataError::custom("unavailable transliterator")
                    .with_debug_context(s)
                    .into()),
            }
        } else {
            let mut data_locale = DataLocale::default();
            data_locale.set_aux(id.parse()?);
            let req = DataRequest {
                locale: &data_locale,
                metadata: Default::default(),
            };
            let rbt = provider.load(req)?.take_payload()?;
            Ok(InternalTransliterator::RuleBased(rbt))
        }
    }

    pub fn borrow(&self) -> BorrowedTransliterator {
        let mut env = LiteMap::new();
        for (k, v) in self.env.iter() {
            match v {
                InternalTransliterator::RuleBased(rbt) => {
                    env.insert(
                        k.into(),
                        BorrowedInternalTransliterator::RuleBased(rbt.get()),
                    );
                }
                InternalTransliterator::NFC(nfc) => {
                    env.insert(k.into(), BorrowedInternalTransliterator::NFC(&nfc));
                }
                InternalTransliterator::Dyn(custom) => {
                    env.insert(
                        k.into(),
                        BorrowedInternalTransliterator::Dyn(custom.as_ref()),
                    );
                }
            }
        }
        BorrowedTransliterator {
            main: self.transliterator.get(),
            env,
        }
    }

    // TODO: somehow incentivize the user to use the borrowed version
    pub fn transliterate(&self, input: &str) -> String {
        self.borrow().transliterate(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let t = Transliterator::try_new("de-t-de-d0-ascii".parse().unwrap()).unwrap();
        let input =
            r"Über ältere Lügner lästern ist sehr a\u{0308}rgerlich. Ja, SEHR ÄRGERLICH! - ꜵ";
        let output =
            "Ueber aeltere Luegner laestern ist sehr aergerlich. Ja, SEHR AERGERLICH! - ao";
        assert_eq!(t.transliterate(input), output);
    }
}
