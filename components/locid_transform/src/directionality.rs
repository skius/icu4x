// This file is part of ICU4X. For terms of use, please see the file
// called LICENSE at the top level of the ICU4X source tree
// (online at: https://github.com/unicode-org/icu4x/blob/main/LICENSE ).

use crate::provider::*;
use crate::Direction;
use crate::{LocaleExpander, LocaleTransformError};
use icu_locid::subtags::Language;
use icu_locid::Locale;
use icu_provider::prelude::*;

/// The `LocaleDirectionality` provides methods to determine the direction of a locale based
/// on [`CLDR`] data.
///
/// # Examples
///
/// ```
/// use icu_locid::locale;
/// use icu_locid_transform::{Direction, LocaleDirectionality};
///
/// let ld = LocaleDirectionality::try_new_unstable(&icu_testdata::unstable())
///     .expect("create failed");
///
/// assert_eq!(ld.get(&locale!("en")), Some(Direction::LeftToRight));
/// ```
///
/// [`CLDR`]: http://cldr.unicode.org/
#[derive(Debug)]
pub struct LocaleDirectionality {
    script_direction: DataPayload<ScriptDirectionV1Marker>,
    expander: LocaleExpander,
}

impl LocaleDirectionality {
    /// A constructor which takes a [`DataProvider`] and creates a [`LocaleDirectionality`].
    ///
    /// [📚 Help choosing a constructor](icu_provider::constructors)
    /// <div class="stab unstable">
    /// ⚠️ The bounds on this function may change over time, including in SemVer minor releases.
    /// </div>
    pub fn try_new_unstable<P>(provider: &P) -> Result<LocaleDirectionality, LocaleTransformError>
    where
        P: DataProvider<ScriptDirectionV1Marker>
            + DataProvider<LikelySubtagsForLanguageV1Marker>
            + DataProvider<LikelySubtagsForScriptRegionV1Marker>
            + ?Sized,
    {
        let expander = LocaleExpander::try_new_unstable(provider)?;
        Self::try_new_with_expander_unstable(provider, expander)
    }

    // Note: This is a custom impl because the bounds on `try_new_unstable` don't suffice
    #[doc = icu_provider::gen_any_buffer_docs!(ANY, icu_provider, Self::try_new_unstable)]
    pub fn try_new_with_any_provider(
        provider: &(impl AnyProvider + ?Sized),
    ) -> Result<LocaleDirectionality, LocaleTransformError> {
        let expander = LocaleExpander::try_new_with_any_provider(provider)?;
        Self::try_new_with_expander_unstable(&provider.as_downcasting(), expander)
    }

    // Note: This is a custom impl because the bounds on `try_new_unstable` don't suffice
    #[doc = icu_provider::gen_any_buffer_docs!(BUFFER, icu_provider, Self::try_new_unstable)]
    #[cfg(feature = "serde")]
    pub fn try_new_with_buffer_provider(
        provider: &(impl BufferProvider + ?Sized),
    ) -> Result<LocaleDirectionality, LocaleTransformError> {
        let expander = LocaleExpander::try_new_with_buffer_provider(provider)?;
        Self::try_new_with_expander_unstable(&provider.as_deserializing(), expander)
    }

    /// Creates a [`LocaleDirectionality`] with a custom [`LocaleExpander`] object.
    ///
    /// For example, use this constructor if you wish to support all languages.
    ///
    /// [📚 Help choosing a constructor](icu_provider::constructors)
    /// <div class="stab unstable">
    /// ⚠️ The bounds on this function may change over time, including in SemVer minor releases.
    /// </div>
    ///
    /// # Examples
    ///
    /// ```
    /// use icu_locid::locale;
    /// use icu_locid_transform::{Direction, LocaleDirectionality, LocaleExpander};
    ///
    /// let ld_default = LocaleDirectionality::try_new_unstable(&icu_testdata::unstable())
    ///     .expect("create failed");
    ///
    /// assert_eq!(ld_default.get(&locale!("jbn")), None);
    ///
    /// let expander = LocaleExpander::try_new_extended_unstable(&icu_testdata::unstable())
    ///     .expect("create failed");
    /// let ld_extended = LocaleDirectionality::try_new_with_expander_unstable(
    ///         &icu_testdata::unstable(),
    ///         expander,
    ///     ).expect("create failed");
    ///
    /// assert_eq!(ld_extended.get(&locale!("jbn")), Some(Direction::RightToLeft));
    /// ```
    pub fn try_new_with_expander_unstable<P>(
        provider: &P,
        expander: LocaleExpander,
    ) -> Result<LocaleDirectionality, LocaleTransformError>
    where
        P: DataProvider<ScriptDirectionV1Marker> + ?Sized,
    {
        let script_direction = provider.load(Default::default())?.take_payload()?;

        Ok(LocaleDirectionality {
            script_direction,
            expander,
        })
    }

    /// Returns the script direction of the given locale.
    ///
    /// # Examples
    ///
    /// ```
    /// use icu_locid::locale;
    /// use icu_locid_transform::{Direction, LocaleDirectionality};
    ///
    /// let ld = LocaleDirectionality::try_new_unstable(&icu_testdata::unstable())
    ///     .expect("create failed");
    ///
    /// assert_eq!(ld.get(&locale!("en-US")), Some(Direction::LeftToRight));
    ///
    /// assert_eq!(ld.get(&locale!("ar")), Some(Direction::RightToLeft));
    ///
    /// assert_eq!(ld.get(&locale!("foo")), None);
    /// ```
    pub fn get(&self, locale: &Locale) -> Option<Direction> {
        let script = locale.id.script.or_else(|| {
            let expander = self.expander.as_borrowed();
            let locale_language = locale.id.language;
            let locale_region = locale.id.region;

            // proceed through _all possible cases_ in order of specificity
            // (borrowed from LocaleExpander::maximize):
            // 1. language + region
            // 2. language
            // 3. region
            // we need to check all cases, because e.g. for "en-US" the default script is associated
            // with "en" but not "en-US"
            if locale_language != Language::UND {
                if let Some(region) = locale_region {
                    // 1. we know both language and region
                    if let Some(script) = expander.get_lr(locale_language, region) {
                        return Some(script);
                    }
                }
                // 2. we know language, but we either do not know region or knowing region did not help
                if let Some((script, _)) = expander.get_l(locale_language) {
                    return Some(script);
                }
            }
            if let Some(region) = locale_region {
                // 3. we know region, but we either do not know language or knowing language did not help
                if let Some((_, script)) = expander.get_r(region) {
                    return Some(script);
                }
            }
            // we could not figure out the script from the given locale
            None
        })?;

        self.script_direction
            .get()
            .rtl
            .get_copied(&script.into_tinystr().to_unvalidated())
    }

    /// Returns true if the given locale is right-to-left.
    ///
    /// See [`LocaleDirectionality::get`] for more information.
    pub fn is_right_to_left(&self, locale: &Locale) -> bool {
        self.get(locale) == Some(Direction::RightToLeft)
    }

    /// Returns true if the given locale is left-to-right.
    ///
    /// See [`LocaleDirectionality::get`] for more information.
    pub fn is_left_to_right(&self, locale: &Locale) -> bool {
        self.get(locale) == Some(Direction::LeftToRight)
    }
}