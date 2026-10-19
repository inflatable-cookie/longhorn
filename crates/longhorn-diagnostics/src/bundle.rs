//! The bundle layout and its redact-last pass.

use std::time::SystemTime;

use longhorn_redact::Redactor;
use longhorn_support_log::{RingRecord, utc_timestamp};

/// The byte column a fact value starts at, counted after the two-space
/// indent: the label, its colon and the padding together fill it. A label at
/// or over this many bytes leaves no padding.
const LABEL_COLUMN: usize = 18;

/// One fact row: a label and its value, rendered on one line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fact {
    /// The row's label, for example `Version`.
    pub label: String,
    /// The row's value.
    pub value: String,
}

impl Fact {
    /// A fact from a label and a value.
    #[must_use]
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }
}

/// One section: a heading and its rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Section {
    /// Fact rows, one per [`Fact`]. An empty list renders the heading only.
    Facts {
        /// The section's heading line.
        heading: String,
        /// The fact rows, in order.
        facts: Vec<Fact>,
    },
    /// List rows, `- item`. An empty list renders the `empty` sentence as
    /// the only list row.
    Items {
        /// The section's heading line.
        heading: String,
        /// The list rows, in order.
        items: Vec<String>,
        /// The sentence rendered when `items` is empty.
        empty: String,
    },
    /// One row per [`RingRecord::line`], indented without a dash. An empty
    /// list renders the `empty` sentence as the only list row.
    Records {
        /// The section's heading line.
        heading: String,
        /// The record rows, oldest first.
        records: Vec<RingRecord>,
        /// The sentence rendered when `records` is empty.
        empty: String,
    },
}

/// Everything one bundle reports, gathered by the host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Facts {
    /// When the bundle was generated. Rendered with
    /// [`longhorn_support_log::utc_timestamp`].
    pub generated_at: SystemTime,
    /// The bundle's title line.
    pub title: String,
    /// The redaction statement under the title.
    pub redaction_statement: String,
    /// The sections, in render order.
    pub sections: Vec<Section>,
}

/// The rendered bundle. The text is already redacted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bundle {
    /// The complete redacted support text, ending with a newline.
    pub text: String,
    /// The generated-at timestamp as it renders in the text.
    pub generated_at: String,
    /// How many `[redacted]` markers the final text holds, including any the
    /// caller's inputs already carried.
    pub redactions: usize,
}

/// Writes interior CR and LF as the two characters `\r` and `\n`, so a text
/// never breaks its line. The same rule contract 025 gives log lines; a
/// backslash is not escaped.
fn single_line(text: &str) -> String {
    text.replace('\r', "\\r").replace('\n', "\\n")
}

fn push_fact_row(out: &mut String, fact: &Fact) {
    let label = single_line(&fact.label);
    out.push_str("  ");
    out.push_str(&label);
    out.push(':');
    for _ in label.len()..LABEL_COLUMN {
        out.push(' ');
    }
    out.push_str(&single_line(&fact.value));
    out.push('\n');
}

fn push_item_row(out: &mut String, item: &str) {
    out.push_str("  - ");
    out.push_str(&single_line(item));
    out.push('\n');
}

fn push_record_row(out: &mut String, record: &RingRecord) {
    out.push_str("  ");
    out.push_str(&record.line());
    out.push('\n');
}

fn push_section(out: &mut String, section: &Section) {
    match section {
        Section::Facts { heading, facts } => {
            out.push_str(&single_line(heading));
            out.push('\n');
            for fact in facts {
                push_fact_row(out, fact);
            }
        }
        Section::Items {
            heading,
            items,
            empty,
        } => {
            out.push_str(&single_line(heading));
            out.push('\n');
            if items.is_empty() {
                push_item_row(out, empty);
            } else {
                for item in items {
                    push_item_row(out, item);
                }
            }
        }
        Section::Records {
            heading,
            records,
            empty,
        } => {
            out.push_str(&single_line(heading));
            out.push('\n');
            if records.is_empty() {
                push_item_row(out, empty);
            } else {
                for record in records {
                    push_record_row(out, record);
                }
            }
        }
    }
}

/// Lays the facts out as one text, escaping interior CR and LF as it goes,
/// then redacts the whole text with one `Redactor::redact` call, so a span
/// that crosses a label and its value is still caught. Because the escape
/// runs first, a seed whose match depends on a raw line break is not caught;
/// contract 026 lists that delta. The same facts through an equivalent
/// redactor render to the same bytes. The crate applies no size bound: the
/// inputs are already bounded by their sources.
#[must_use]
pub fn render(redactor: &Redactor, facts: &Facts) -> Bundle {
    let generated_at = utc_timestamp(facts.generated_at);
    let mut out = String::new();
    out.push_str(&single_line(&facts.title));
    out.push('\n');
    out.push_str("Generated: ");
    out.push_str(&generated_at);
    out.push('\n');
    out.push_str(&single_line(&facts.redaction_statement));
    out.push('\n');
    out.push('\n');
    let mut sections = facts.sections.iter();
    if let Some(section) = sections.next() {
        push_section(&mut out, section);
        for section in sections {
            out.push('\n');
            push_section(&mut out, section);
        }
    }
    let redaction = redactor.redact(&out);
    Bundle {
        text: redaction.text,
        generated_at,
        redactions: redaction.count,
    }
}
