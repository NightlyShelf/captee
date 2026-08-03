//! Parsing of compiler diagnostics into UI-independent values.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSpan {
    pub path: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub span: Option<SourceSpan>,
}

/// Parses Typst short-format lines such as `error: main.typ:3:5: unknown name`.
pub fn parse_diagnostics(output: &str) -> Vec<Diagnostic> {
    output.lines().filter_map(parse_line).collect()
}

fn parse_line(line: &str) -> Option<Diagnostic> {
    let line = line.trim();
    let (severity, remainder) = line.split_once(':')?;
    let severity = match severity.trim() {
        "error" => DiagnosticSeverity::Error,
        "warning" => DiagnosticSeverity::Warning,
        "info" | "note" => DiagnosticSeverity::Info,
        _ => return None,
    };
    let remainder = remainder.trim();
    let (span, message) = parse_location(remainder);
    if message.is_empty() {
        return None;
    }
    Some(Diagnostic { severity, message: message.to_owned(), span })
}

fn parse_location(value: &str) -> (Option<SourceSpan>, &str) {
    for (first, character) in value.char_indices() {
        if character != ':' || first == 0 {
            continue;
        }
        let after_line = &value[first + 1..];
        let Some(second) = after_line.find(':') else {
            continue;
        };
        let after_column = &after_line[second + 1..];
        let Some(third) = after_column.find(':') else {
            continue;
        };
        let line_number = &after_line[..second];
        let column_number = &after_column[..third];
        let Ok(line_number) = line_number.parse::<u32>() else {
            continue;
        };
        let Ok(column_number) = column_number.parse::<u32>() else {
            continue;
        };
        let path = value[..first].trim();
        let message = after_column[third + 1..].trim();
        if path.is_empty() || message.is_empty() {
            continue;
        }
        return (
            Some(SourceSpan { path: path.to_owned(), line: line_number, column: column_number }),
            message,
        );
    }
    (None, value.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_location_and_severity() {
        let diagnostics = parse_diagnostics("error: main.typ:3:5: unknown name");
        assert_eq!(
            diagnostics,
            vec![Diagnostic {
                severity: DiagnosticSeverity::Error,
                message: "unknown name".to_owned(),
                span: Some(SourceSpan { path: "main.typ".to_owned(), line: 3, column: 5 }),
            }]
        );
    }

    #[test]
    fn retains_diagnostics_without_locations() {
        let diagnostics =
            parse_diagnostics("warning: deprecated syntax\nnote: consider using a function");
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(diagnostics[0].severity, DiagnosticSeverity::Warning);
        assert_eq!(diagnostics[0].span, None);
        assert_eq!(diagnostics[1].severity, DiagnosticSeverity::Info);
    }

    #[test]
    fn skips_unrecognized_and_empty_lines() {
        let diagnostics =
            parse_diagnostics("\ncompiler output\nerror: \nerror: main.typ:1:1: broken");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message, "broken");
    }
}
