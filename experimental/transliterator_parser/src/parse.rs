// This file is part of ICU4X. For terms of use, please see the file
// called LICENSE at the top level of the ICU4X source tree
// (online at: https://github.com/unicode-org/icu4x/blob/main/LICENSE ).

// TODO: remove this
#![allow(unused)]

use std::collections::HashMap;
use std::{iter::Peekable, str::CharIndices};

use icu_collections::{
    codepointinvlist::CodePointInversionList,
    codepointinvliststringlist::CodePointInversionListAndStringList,
};
use icu_properties::provider::*;
use icu_properties::sets::{load_pattern_white_space, load_xid_continue, load_xid_start};
use icu_provider::prelude::*;

/// The kind of error that occurred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseErrorKind {
    /// An unexpected character was encountered. This variant implies the other variants
    /// (notably `UnknownProperty` and `Unimplemented`) do not apply.
    UnexpectedChar(char),
    /// A reference to an unknown variable.
    UnknownVariable,
    /// The source is an incomplete unicode set.
    Eof,
    /// Something unexpected went wrong with our code. Please file a bug report on GitHub.
    Internal,
    /// The provided syntax is not supported by us. Note that unknown properties will return the
    /// `UnknownProperty` variant, not this one.
    Unimplemented,
    /// The provided escape sequence is not a valid Unicode code point or represents too many
    /// code points.
    InvalidEscape,
    /// The provided transform ID is invalid.
    InvalidId,
    /// Invalid UnicodeSet syntax. See `icu_unicodeset_parser`'s [`ParseErrorKind`](icu_unicodeset_parser::ParseErrorKind).
    UnicodeSetError(icu_unicodeset_parser::ParseErrorKind),
}
use ParseErrorKind as PEK;

impl ParseErrorKind {
    fn with_offset(self, offset: usize) -> ParseError {
        ParseError {
            offset: Some(offset),
            kind: self,
        }
    }
}

/// The error type returned by the `parse` functions in this crate.
///
/// See [`ParseError::fmt_with_source`] for pretty-printing and [`ParseErrorKind`] of the
/// different types of errors represented by this struct.
#[derive(Debug, Clone, Copy)]
pub struct ParseError {
    // offset is the index to an arbitrary byte in the last character in the source that makes sense
    // to display as location for the error, e.g., the unexpected character itself or
    // for an unknown property name the last character of the name.
    offset: Option<usize>,
    kind: ParseErrorKind,
}

impl From<ParseErrorKind> for ParseError {
    fn from(kind: ParseErrorKind) -> Self {
        ParseError { offset: None, kind }
    }
}

impl From<icu_unicodeset_parser::ParseError> for ParseError {
    fn from(e: icu_unicodeset_parser::ParseError) -> Self {
        ParseError {
            offset: todo!("e.offset()"),
            kind: PEK::UnicodeSetError(e.kind()),
        }
    }
}

type Result<T, E = ParseError> = core::result::Result<T, E>;

type UnicodeSet = CodePointInversionListAndStringList<'static>;

#[derive(Debug, Clone, Copy)]
pub(crate) enum QuantifierKind {
    // ?
    ZeroOrOne,
    // *
    ZeroOrMore,
    // +
    OneOrMore,
}

// source-target/variant
#[derive(Debug, Clone)]
pub(crate) struct BasicId {
    source: String,
    target: String,
    variant: String,
}

impl Default for BasicId {
    fn default() -> Self {
        Self {
            source: "Any".to_string(),
            target: "Null".to_string(),
            variant: "".to_string(),
        }
    }
}

// [set] source-target/variant
#[derive(Debug, Clone)]
pub(crate) struct SingleId {
    filter: Option<UnicodeSet>,
    basic_id: BasicId,
}

#[derive(Debug, Clone)]
pub(crate) enum Element {
    // Examples:
    //  - hello\ world
    //  - 'hello world'
    Literal(String),
    // Example: $my_var
    VariableRef(String),
    // Example: $12
    BackRef(u32),
    // Examples:
    //  - <element>?
    //  - <element>*
    //  - <element>+
    // note: Box<Element> instead of Section, because a quantifier only ever refers to the immediately preceding element.
    // segments or variable refs are used to group multiple elements together.
    Quantifier(QuantifierKind, Box<Element>),
    // Example: (<element> <element> ...)
    Segment(Section),
    // Example: [:^L:]
    UnicodeSet(UnicodeSet),
    // Example: &[a-z] Any-Remove(<element> <element> ...)
    // single id, function arguments
    FunctionCall(SingleId, Section),
    // '|', only valid on the output side
    Cursor,
    // '@', only valid on the output side
    Placeholder,
}

type Section = Vec<Element>;

#[derive(Debug, Clone)]
pub(crate) struct HalfRule {
    ante: Section,
    key: Section,
    post: Section,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Dir {
    Forward,
    Backward,
    Both,
}

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub(crate) enum Rule {
    GlobalFilter(UnicodeSet),
    GlobalInverseFilter(UnicodeSet),
    // forward and backward IDs.
    // "A (B)" is Transform(A, Some(B)),
    // "(B)" is Transform(Null, Some(B)),
    // "A" is Transform(A, None),
    // "A ()" is Transform(A, Some(Null))
    Transform(SingleId, Option<SingleId>),
    Conversion(HalfRule, Dir, HalfRule),
    VariableDefinition(String, Section),
}

type ParseVariableMap = HashMap<String, Section>;

struct TransliteratorParser<'a, 'b, P: ?Sized> {
    // TODO: if we also keep a reference to source we can pass that onto the UnicodeSet parser without allocating
    iter: &'a mut Peekable<CharIndices<'b>>,
    variable_map: ParseVariableMap,
    // for variable identifiers (XID Start, XID Continue)
    xid_start: &'a CodePointInversionList<'a>,
    xid_continue: &'a CodePointInversionList<'a>,
    // for skipped whitespace (Pattern White Space)
    pat_ws: &'a CodePointInversionList<'a>,
    property_provider: &'a P,
}

impl<'a, 'b, P> TransliteratorParser<'a, 'b, P>
where
    P: ?Sized
        + DataProvider<AsciiHexDigitV1Marker>
        + DataProvider<AlphabeticV1Marker>
        + DataProvider<BidiControlV1Marker>
        + DataProvider<BidiMirroredV1Marker>
        + DataProvider<CaseIgnorableV1Marker>
        + DataProvider<CasedV1Marker>
        + DataProvider<ChangesWhenCasefoldedV1Marker>
        + DataProvider<ChangesWhenCasemappedV1Marker>
        + DataProvider<ChangesWhenLowercasedV1Marker>
        + DataProvider<ChangesWhenNfkcCasefoldedV1Marker>
        + DataProvider<ChangesWhenTitlecasedV1Marker>
        + DataProvider<ChangesWhenUppercasedV1Marker>
        + DataProvider<DashV1Marker>
        + DataProvider<DefaultIgnorableCodePointV1Marker>
        + DataProvider<DeprecatedV1Marker>
        + DataProvider<DiacriticV1Marker>
        + DataProvider<EmojiV1Marker>
        + DataProvider<EmojiComponentV1Marker>
        + DataProvider<EmojiModifierV1Marker>
        + DataProvider<EmojiModifierBaseV1Marker>
        + DataProvider<EmojiPresentationV1Marker>
        + DataProvider<ExtendedPictographicV1Marker>
        + DataProvider<ExtenderV1Marker>
        + DataProvider<GraphemeBaseV1Marker>
        + DataProvider<GraphemeExtendV1Marker>
        + DataProvider<HexDigitV1Marker>
        + DataProvider<IdsBinaryOperatorV1Marker>
        + DataProvider<IdsTrinaryOperatorV1Marker>
        + DataProvider<IdContinueV1Marker>
        + DataProvider<IdStartV1Marker>
        + DataProvider<IdeographicV1Marker>
        + DataProvider<JoinControlV1Marker>
        + DataProvider<LogicalOrderExceptionV1Marker>
        + DataProvider<LowercaseV1Marker>
        + DataProvider<MathV1Marker>
        + DataProvider<NoncharacterCodePointV1Marker>
        + DataProvider<PatternSyntaxV1Marker>
        + DataProvider<PatternWhiteSpaceV1Marker>
        + DataProvider<QuotationMarkV1Marker>
        + DataProvider<RadicalV1Marker>
        + DataProvider<RegionalIndicatorV1Marker>
        + DataProvider<SentenceTerminalV1Marker>
        + DataProvider<SoftDottedV1Marker>
        + DataProvider<TerminalPunctuationV1Marker>
        + DataProvider<UnifiedIdeographV1Marker>
        + DataProvider<UppercaseV1Marker>
        + DataProvider<VariationSelectorV1Marker>
        + DataProvider<WhiteSpaceV1Marker>
        + DataProvider<XidContinueV1Marker>
        + DataProvider<GeneralCategoryMaskNameToValueV1Marker>
        + DataProvider<GeneralCategoryV1Marker>
        + DataProvider<ScriptNameToValueV1Marker>
        + DataProvider<ScriptV1Marker>
        + DataProvider<ScriptWithExtensionsPropertyV1Marker>
        + DataProvider<XidStartV1Marker>,
{
    // initiates a line comment
    const COMMENT: char = '#';
    // terminates a line comment
    const COMMENT_END: char = '\n';
    // terminates a rule
    const RULE_END: char = ';';
    // initiates a filter or transform rule, as part of '::'
    const SPECIAL_START: char = ':';
    // initiates a UnicodeSet
    // TODO: does \p (and \P) work to initiate a UnicodeSet?
    const SET_START: char = '[';
    // initiates a segment or the reverse portion of an ID
    const OPEN_PAREN: char = '(';
    // terminates a segment or the reverse portion of an ID
    const CLOSE_PAREN: char = ')';
    // separates source and target of an ID
    const ID_SEP: char = '-';
    // separates variant from ID
    const VARIANT_SEP: char = '/';

    fn new(
        iter: &'a mut Peekable<CharIndices<'b>>,
        xid_start: &'a CodePointInversionList<'a>,
        xid_continue: &'a CodePointInversionList<'a>,
        pat_ws: &'a CodePointInversionList<'a>,
        provider: &'a P,
    ) -> Self {
        Self {
            iter,
            variable_map: Default::default(),
            xid_start,
            xid_continue,
            pat_ws,
            property_provider: provider,
        }
    }

    fn parse_rules(&mut self) -> Result<Vec<Rule>> {
        let mut rules = Vec::new();

        loop {
            self.skip_whitespace();
            if self.iter.peek().is_none() {
                break;
            }
            // we skipped whitespace and comments, so any other chars must be part of a rule
            rules.push(self.parse_rule()?);
        }

        Ok(rules)
    }

    // expects a rule
    fn parse_rule(&mut self) -> Result<Rule> {
        match self.must_peek_char()? {
            Self::SPECIAL_START => self.parse_filter_or_transform_rule(),
            // must be a conversion or variable rule
            _ => self.parse_conversion_or_variable_rule(),
        }
    }

    // any rules starting with '::'
    fn parse_filter_or_transform_rule(&mut self) -> Result<Rule> {
        // Syntax:
        // '::' <unicodeset> ';'                  # global filter
        // '::' '(' <unicodeset> ')' ';'          # global inverse filter
        // '::' <single-id> (<single-id>)? ';'    # transform rule

        self.consume(Self::SPECIAL_START)?;
        self.consume(Self::SPECIAL_START)?;

        // because all three options can start with a UnicodeSet, we just try to parse everything
        // into options, and assemble at the end

        let (forward_filter, forward_basic_id, reverse_filter, reverse_basic_id, has_reverse) =
            self.parse_filter_or_transform_rule_parts()?;

        // the offset of ';'
        let meta_err_offset = self.must_peek_index()?;
        self.consume(Self::RULE_END)?;

        // try to assemble the rule
        // first try global filters
        match (
            forward_filter.is_some(),
            forward_basic_id.is_some(),
            reverse_filter.is_some(),
            reverse_basic_id.is_some(),
        ) {
            (true, false, false, false) => {
                // safety: by match, forward_filter.is_some() is true
                #[allow(clippy::panicking_unwrap)]
                return Ok(Rule::GlobalFilter(forward_filter.unwrap()));
            }
            (false, false, true, false) => {
                // safety: by match, reverse_filter.is_some() is true
                #[allow(clippy::panicking_unwrap)]
                return Ok(Rule::GlobalInverseFilter(reverse_filter.unwrap()));
            }
            _ => {}
        }

        // if this is not a global filter rule, this must be a transform rule

        // either forward_basic_id or reverse_basic_id must be nonempty
        if forward_basic_id.is_none() && reverse_basic_id.is_none() {
            return Err(PEK::InvalidId.with_offset(meta_err_offset));
        }

        if !has_reverse {
            // we must have a forward id due to:
            //  1. !has_reverse implying reverse_basic_id.is_none()
            //  2. the above none checks implying forward_basic_id.is_some()
            // because this is difficult to verify, returning a PEK::Internal anyway
            // instead of unwrapping, despite technically being unnecessary
            let forward_basic_id = forward_basic_id.ok_or(PEK::Internal)?;
            return Ok(Rule::Transform(
                SingleId {
                    basic_id: forward_basic_id,
                    filter: forward_filter,
                },
                None,
            ));
        }

        if forward_filter.is_some() && forward_basic_id.is_none()
            || reverse_filter.is_some() && reverse_basic_id.is_none()
        {
            // cannot have a filter without a basic id
            return Err(PEK::InvalidId.with_offset(meta_err_offset));
        }

        // an empty forward rule, such as ":: (R) ;" is equivalent to ":: Any-Null (R) ;"
        let forward_basic_id = forward_basic_id.unwrap_or(BasicId::default());
        // an empty reverse rule, such as ":: F () ;" is equivalent to ":: F (Any-Null) ;"
        let reverse_basic_id = reverse_basic_id.unwrap_or(BasicId::default());

        let forward_single_id = SingleId {
            basic_id: forward_basic_id,
            filter: forward_filter,
        };
        let reverse_single_id = SingleId {
            basic_id: reverse_basic_id,
            filter: reverse_filter,
        };

        Ok(Rule::Transform(forward_single_id, Some(reverse_single_id)))
    }

    fn parse_filter_or_transform_rule_parts(
        &mut self,
    ) -> Result<(
        Option<UnicodeSet>,
        Option<BasicId>,
        Option<UnicodeSet>,
        Option<BasicId>,
        bool,
    )> {
        // until we encounter a Self::OPEN_PAREN, we're parsing the forward single id or global filter
        let mut forward_filter = None;
        let mut forward_basic_id = None;

        let mut reverse_filter = None;
        let mut reverse_basic_id = None;

        let has_reverse;

        // parse forward things, i.e., everything until Self::OPEN_PAREN
        loop {
            self.skip_whitespace();
            match self.must_peek_char()? {
                Self::OPEN_PAREN => {
                    // we're done parsing forward things
                    has_reverse = true;
                    break;
                }
                Self::RULE_END => {
                    // we're done parsing completely
                    has_reverse = false;
                    break;
                }
                Self::SET_START => {
                    forward_filter = Some(self.parse_unicode_set()?);
                }
                _ => {
                    forward_basic_id = Some(self.parse_basic_id()?);
                }
            }
        }

        // if we have a reverse, parse it
        if has_reverse {
            self.consume(Self::OPEN_PAREN)?;
            loop {
                self.skip_whitespace();
                match self.must_peek_char()? {
                    Self::CLOSE_PAREN => {
                        // we're done parsing reverse things
                        break;
                    }
                    Self::SET_START => {
                        reverse_filter = Some(self.parse_unicode_set()?);
                    }
                    _ => {
                        reverse_basic_id = Some(self.parse_basic_id()?);
                    }
                }
            }
        }

        Ok((
            forward_filter,
            forward_basic_id,
            reverse_filter,
            reverse_basic_id,
            has_reverse,
        ))
    }

    fn parse_conversion_or_variable_rule(&mut self) -> Result<Rule> {
        // Syntax:
        // <variable_ref> '=' <section> ';'           # variable rule
        // <half-rule> <direction> <half-rule> ';'    # conversion rule

        todo!()
    }

    fn parse_single_id(&mut self) -> Result<SingleId> {
        // Syntax:
        // <unicodeset>? <basic-id>

        todo!()
    }

    // TODO: factor this out for runtime ID parsing?
    fn parse_basic_id(&mut self) -> Result<BasicId> {
        // Syntax:
        // <single-id> ('-' <single-id>)? ('/' <single-id>)?

        todo!()
    }

    fn parse_half_rule(&mut self) -> Result<HalfRule> {
        // Syntax:
        // (<section> '{')? <section> ('}' <section>)?

        todo!()
    }

    fn parse_dir(&mut self) -> Result<Dir> {
        // Syntax:
        // '<' | '>' | '<>' | '→' | '←' | '↔'

        todo!()
    }

    fn parse_section(&mut self) -> Result<Section> {
        todo!()
    }

    fn parse_element(&mut self) -> Result<Element> {
        todo!()
    }

    fn parse_variable_or_backref(&mut self) -> Result<Element> {
        todo!()
    }

    fn parse_quoted_literal(&mut self) -> Result<String> {
        todo!()
    }

    // parses all supported escapes. code is somewhat duplicated from icu_unicodeset_parser
    fn parse_escaped_char(&mut self) -> Result<char> {
        todo!()
    }

    fn parse_segment(&mut self) -> Result<Element> {
        todo!()
    }

    fn parse_unicode_set(&mut self) -> Result<UnicodeSet> {
        let mut set = String::new();
        self.consume('[')?;
        set.push('[');

        let mut depth = 1;
        let mut escaped = false;

        loop {
            let c = self.must_next_char()?;
            set.push(c);
            match c {
                '[' if !escaped => depth += 1,
                ']' if !escaped => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                '\\' if !escaped => escaped = true,
                _ => escaped = false,
            }
        }

        let set = icu_unicodeset_parser::parse_unstable(&set, self.property_provider)?;

        Ok(set)
    }

    fn parse_function_call(&mut self) -> Result<Element> {
        todo!()
    }

    fn consume(&mut self, expected: char) -> Result<()> {
        match self.must_next()? {
            (offset, c) if c != expected => Err(PEK::UnexpectedChar(c).with_offset(offset)),
            _ => Ok(()),
        }
    }

    // skips whitespace and comments
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek_char() {
            if c == Self::COMMENT {
                self.skip_until(Self::COMMENT_END);
            }
            if !self.pat_ws.contains(c) {
                break;
            }
            self.iter.next();
        }
    }

    // skips until the next occurrence of c, which is also consumed
    fn skip_until(&mut self, end: char) {
        while let Some((_, c)) = self.iter.next() {
            if c == end {
                break;
            }
        }
    }

    fn peek_char(&mut self) -> Option<char> {
        self.iter.peek().map(|(_, c)| *c)
    }

    fn peek_offset(&mut self) -> Option<usize> {
        self.iter.peek().map(|(i, _)| *i)
    }

    // use this whenever an empty iterator would imply an Eof error
    fn must_next(&mut self) -> Result<(usize, char)> {
        self.iter.next().ok_or(PEK::Eof.into())
    }

    // see must_next
    fn must_next_char(&mut self) -> Result<char> {
        self.must_next().map(|(_, c)| c)
    }

    // use this whenever an empty iterator would imply an Eof error
    fn must_peek(&mut self) -> Result<(usize, char)> {
        self.iter.peek().copied().ok_or(PEK::Eof.into())
    }

    // must_peek, but looks two chars ahead. use sparingly
    fn must_peek_double(&mut self) -> Result<(usize, char)> {
        let mut copy = self.iter.clone();
        copy.next();
        copy.next().ok_or(PEK::Eof.into())
    }

    // see must_peek
    fn must_peek_char(&mut self) -> Result<char> {
        self.must_peek().map(|(_, c)| c)
    }

    // see must_peek
    fn must_peek_index(&mut self) -> Result<usize> {
        self.must_peek().map(|(idx, _)| idx)
    }
}

#[cfg(feature = "compiled_data")]
pub(crate) fn parse(source: &str) -> Result<Vec<Rule>> {
    parse_unstable(source, &icu_properties::provider::Baked)
}

pub(crate) fn parse_unstable<P>(source: &str, provider: &P) -> Result<Vec<Rule>>
where
    P: ?Sized
        + DataProvider<AsciiHexDigitV1Marker>
        + DataProvider<AlphabeticV1Marker>
        + DataProvider<BidiControlV1Marker>
        + DataProvider<BidiMirroredV1Marker>
        + DataProvider<CaseIgnorableV1Marker>
        + DataProvider<CasedV1Marker>
        + DataProvider<ChangesWhenCasefoldedV1Marker>
        + DataProvider<ChangesWhenCasemappedV1Marker>
        + DataProvider<ChangesWhenLowercasedV1Marker>
        + DataProvider<ChangesWhenNfkcCasefoldedV1Marker>
        + DataProvider<ChangesWhenTitlecasedV1Marker>
        + DataProvider<ChangesWhenUppercasedV1Marker>
        + DataProvider<DashV1Marker>
        + DataProvider<DefaultIgnorableCodePointV1Marker>
        + DataProvider<DeprecatedV1Marker>
        + DataProvider<DiacriticV1Marker>
        + DataProvider<EmojiV1Marker>
        + DataProvider<EmojiComponentV1Marker>
        + DataProvider<EmojiModifierV1Marker>
        + DataProvider<EmojiModifierBaseV1Marker>
        + DataProvider<EmojiPresentationV1Marker>
        + DataProvider<ExtendedPictographicV1Marker>
        + DataProvider<ExtenderV1Marker>
        + DataProvider<GraphemeBaseV1Marker>
        + DataProvider<GraphemeExtendV1Marker>
        + DataProvider<HexDigitV1Marker>
        + DataProvider<IdsBinaryOperatorV1Marker>
        + DataProvider<IdsTrinaryOperatorV1Marker>
        + DataProvider<IdContinueV1Marker>
        + DataProvider<IdStartV1Marker>
        + DataProvider<IdeographicV1Marker>
        + DataProvider<JoinControlV1Marker>
        + DataProvider<LogicalOrderExceptionV1Marker>
        + DataProvider<LowercaseV1Marker>
        + DataProvider<MathV1Marker>
        + DataProvider<NoncharacterCodePointV1Marker>
        + DataProvider<PatternSyntaxV1Marker>
        + DataProvider<PatternWhiteSpaceV1Marker>
        + DataProvider<QuotationMarkV1Marker>
        + DataProvider<RadicalV1Marker>
        + DataProvider<RegionalIndicatorV1Marker>
        + DataProvider<SentenceTerminalV1Marker>
        + DataProvider<SoftDottedV1Marker>
        + DataProvider<TerminalPunctuationV1Marker>
        + DataProvider<UnifiedIdeographV1Marker>
        + DataProvider<UppercaseV1Marker>
        + DataProvider<VariationSelectorV1Marker>
        + DataProvider<WhiteSpaceV1Marker>
        + DataProvider<XidContinueV1Marker>
        + DataProvider<GeneralCategoryMaskNameToValueV1Marker>
        + DataProvider<GeneralCategoryV1Marker>
        + DataProvider<ScriptNameToValueV1Marker>
        + DataProvider<ScriptV1Marker>
        + DataProvider<ScriptWithExtensionsPropertyV1Marker>
        + DataProvider<XidStartV1Marker>,
{
    let mut iter = source.char_indices().peekable();

    let xid_start = load_xid_start(provider).map_err(|_| PEK::Internal)?;
    let xid_start_list = xid_start.to_code_point_inversion_list();
    let xid_continue = load_xid_continue(provider).map_err(|_| PEK::Internal)?;
    let xid_continue_list = xid_continue.to_code_point_inversion_list();

    let pat_ws = load_pattern_white_space(provider).map_err(|_| PEK::Internal)?;
    let pat_ws_list = pat_ws.to_code_point_inversion_list();

    let mut parser = TransliteratorParser::new(
        &mut iter,
        &xid_start_list,
        &xid_continue_list,
        &pat_ws_list,
        provider,
    );
    parser.parse_rules()
}
