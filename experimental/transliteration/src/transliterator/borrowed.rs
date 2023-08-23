// This file is part of ICU4X. For terms of use, please see the file
// called LICENSE at the top level of the ICU4X source tree
// (online at: https://github.com/unicode-org/icu4x/blob/main/LICENSE ).

use super::{CustomTransliterator, Filter, FilterChain, NFCTransliterator};
use crate::provider::{FunctionCall, Rule, RuleBasedTransliterator, RuleULE, SimpleId, VarTable};
use alloc::borrow::Cow;
use alloc::string::String;
use alloc::vec::Vec;
use core::str;
use icu_collections::codepointinvlist::CodePointInversionList;
use icu_collections::codepointinvliststringlist::CodePointInversionListAndStringList;
use litemap::LiteMap;
use zerofrom::ZeroFrom;
use zerovec::VarZeroSlice;

pub(super) enum BorrowedInternalTransliterator<'a> {
    RuleBased(&'a RuleBasedTransliterator<'a>),
    NFC(&'a NFCTransliterator),
    Dyn(&'a dyn CustomTransliterator),
}

impl<'a> BorrowedInternalTransliterator<'a> {
    fn transliterate(&self, input: &str, filter: &FilterChain, env: &Env<'a>) -> String {
        match self {
            Self::RuleBased(rbt) => rbt.transliterate(input, filter, env),
            // TODO: internal hardcoded transliterators
            Self::NFC(_nfc) => input.to_string(),
            Self::Dyn(custom) => custom.transliterate(input, filter),
        }
    }
}

type Env<'a> = LiteMap<Cow<'a, str>, BorrowedInternalTransliterator<'a>>;

pub(super) struct BorrowedTransliterator<'a> {
    pub(super) main: &'a RuleBasedTransliterator<'a>,
    pub(super) env: Env<'a>,
}

impl<'a> BorrowedTransliterator<'a> {
    pub fn transliterate(&self, input: &str) -> String {
        self.main
            .transliterate(input, &FilterChain::new(), &self.env)
    }
}

impl<'a> RuleBasedTransliterator<'a> {
    fn transliterate(&self, input: &str, filter: &FilterChain, env: &Env<'a>) -> String {
        debug_assert_eq!(self.id_group_list.len(), self.rule_group_list.len());
        let filter = filter.add(&self.filter);

        let mut buf = input.as_bytes().to_vec();

        // first: process the groups in order, i.e., id_group_list[0], rule_group_list[0], id_group_list[1], rule_group_list[1], ...
        for (id_group, rule_group) in self.id_group_list.iter().zip(self.rule_group_list.iter()) {
            // first handle id_group
            for single_id in id_group.iter() {
                let id = SimpleId::zero_from(single_id);
                let transliterated = self.transliterate_nested(
                    id,
                    unsafe { str::from_utf8_unchecked(&buf[..]) },
                    &filter,
                    env,
                );
                buf = transliterated.into_bytes();
            }

            // then handle rule_group
            let transliterated = self.transliterate_rule_group(
                rule_group,
                unsafe { str::from_utf8_unchecked(&buf[..]) },
                &filter,
                env,
            );
            buf = transliterated.into_bytes();
        }

        unsafe { String::from_utf8_unchecked(buf) }
    }

    fn transliterate_nested(
        &self,
        id: SimpleId<'a>,
        input: &str,
        filter: &FilterChain,
        env: &Env<'a>,
    ) -> String {
        let filter = filter.add(&id.filter);
        // this get succeeds for valid RBT serializations
        if let Some(transliterator) = env.get(&id.id) {
            return transliterator.transliterate(input, &filter, env);
        }
        // GIGO, we don't want to panic
        String::new()
    }

    fn transliterate_rule_group(
        &self,
        rules: &VarZeroSlice<RuleULE>,
        input: &str,
        filter: &FilterChain,
        env: &Env<'a>,
    ) -> String {
        // for (start_index, run_length) in filtered_runs...

        for rule in rules.iter() {
            let rule = Rule::zero_from(rule);
            let transliterated = self.transliterate_rule(rule, input, env);
        }

        String::new()
    }

    fn transliterate_rule(&self, rule: Rule<'a>, input: &str, env: &Env<'a>) -> String {
        String::new()
    }

    // returns the byte-length of the run after transliteration
    fn transliterate_run(
        &self,
        inout: &mut Vec<u8>,
        start_index: usize,
        run_length: usize,
        env: &Env<'a>,
    ) -> usize {
        let stop_when_remaining_length_is = inout.len() - start_index - run_length;
        let mut cursor = start_index;
        let mut remaining_length = run_length;
        0
    }
}

enum QuantifierKind {
    ZeroOrOne,
    ZeroOrMore,
    OneOrMore,
}

enum SpecialMatcher<'a> {
    Compound(&'a str),
    Quantifier(QuantifierKind, &'a str),
    Segment(&'a str),
    UnicodeSet(CodePointInversionListAndStringList<'a>),
    AnchorStart,
    AnchorEnd,
}

enum SpecialReplacer<'a> {
    Compound(&'a str),
    FunctionCall(FunctionCall<'a>),
    BackReference(u32),
}

enum VarTableElement<'a> {
    Compound(&'a str),
    Quantifier(QuantifierKind, &'a str),
    Segment(&'a str),
    UnicodeSet(CodePointInversionListAndStringList<'a>),
    FunctionCall(FunctionCall<'a>),
    BackReference(u32),
    AnchorStart,
    AnchorEnd,
}

impl<'a> VarTableElement<'a> {
    fn to_replacer(self) -> Option<SpecialReplacer<'a>> {
        Some(match self {
            VarTableElement::Compound(elt) => SpecialReplacer::Compound(elt),
            VarTableElement::FunctionCall(elt) => SpecialReplacer::FunctionCall(elt),
            VarTableElement::BackReference(elt) => SpecialReplacer::BackReference(elt),
            _ => return None,
        })
    }

    fn to_matcher(self) -> Option<SpecialMatcher<'a>> {
        Some(match self {
            VarTableElement::Compound(elt) => SpecialMatcher::Compound(elt),
            VarTableElement::Quantifier(kind, elt) => SpecialMatcher::Quantifier(kind, elt),
            VarTableElement::Segment(elt) => SpecialMatcher::Segment(elt),
            VarTableElement::UnicodeSet(elt) => SpecialMatcher::UnicodeSet(elt),
            VarTableElement::AnchorEnd => SpecialMatcher::AnchorEnd,
            VarTableElement::AnchorStart => SpecialMatcher::AnchorStart,
            _ => return None,
        })
    }
}

impl<'a> VarTable<'a> {
    // TODO: these must be the same as during datagen. Find some place to define them *once*
    const BASE: u32 = '\u{F0000}' as u32;
    const MAX_DYNAMIC: u32 = '\u{FFFF0}' as u32;
    const RESERVED_ANCHOR_START: u32 = '\u{FFFFC}' as u32;
    const RESERVED_ANCHOR_END: u32 = '\u{FFFFD}' as u32;

    fn lookup(&'a self, query: char) -> Option<VarTableElement<'a>> {
        let query = query as u32;
        if query < Self::BASE {
            return None;
        }
        if query > Self::MAX_DYNAMIC {
            return match query {
                Self::RESERVED_ANCHOR_END => Some(VarTableElement::AnchorEnd),
                Self::RESERVED_ANCHOR_START => Some(VarTableElement::AnchorStart),
                _ => None,
            };
        }
        let idx = query - Self::BASE;
        let mut idx = idx as usize;

        // TODO: these lookups must be in the same order as during datagen. Best way to enforce this?
        // note: might be worth trying to speed up these lookups by binary searching?
        let mut next_base = self.compounds.len();
        if idx < next_base {
            return Some(VarTableElement::Compound(&self.compounds[idx]));
        }
        // no underflow for all these idx subtractions, as idx is always >= next_base
        idx -= next_base;
        next_base = self.quantifiers_opt.len();
        if idx < next_base {
            return Some(VarTableElement::Quantifier(
                QuantifierKind::ZeroOrOne,
                &self.quantifiers_opt[idx],
            ));
        }
        idx -= next_base;
        next_base = self.quantifiers_kleene.len();
        if idx < next_base {
            return Some(VarTableElement::Quantifier(
                QuantifierKind::ZeroOrMore,
                &self.quantifiers_kleene[idx],
            ));
        }
        idx -= next_base;
        next_base = self.quantifiers_kleene_plus.len();
        if idx < next_base {
            return Some(VarTableElement::Quantifier(
                QuantifierKind::OneOrMore,
                &self.quantifiers_kleene_plus[idx],
            ));
        }
        idx -= next_base;
        next_base = self.segments.len();
        if idx < next_base {
            return Some(VarTableElement::Segment(&self.segments[idx]));
        }
        idx -= next_base;
        next_base = self.unicode_sets.len();
        if idx < next_base {
            return Some(VarTableElement::UnicodeSet(
                CodePointInversionListAndStringList::zero_from(&self.unicode_sets[idx]),
            ));
        }
        idx -= next_base;
        next_base = self.function_calls.len();
        if idx < next_base {
            return Some(VarTableElement::FunctionCall(FunctionCall::zero_from(
                &self.function_calls[idx],
            )));
        }
        idx -= next_base;
        // idx must be a backreference (an u32 encoded as <itself> indices past the last valid VarZeroVec index)
        // usize -> u32 conversion is valid because `idx` started out as u32 and was only subtracted from
        Some(VarTableElement::BackReference(idx as u32))
    }

    fn lookup_matcher(&'a self, query: char) -> Option<SpecialMatcher<'a>> {
        let elt = self.lookup(query)?;
        elt.to_matcher()
    }

    fn lookup_replacer(&'a self, query: char) -> Option<SpecialReplacer<'a>> {
        let elt = self.lookup(query)?;
        elt.to_replacer()
    }
}

// struct OngoingTransliteration<'a, 'b> {
//     input: &'a [u8],
//     input_processed: usize,
//     output: &'b mut Vec<u8>,
//     output_
// }
