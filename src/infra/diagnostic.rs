use crate::infra::location::Location;
use crate::infra::result::{FelicoError, FelicoResult};
use crate::infra::source_file::SourceFileHandle;
use miette::{
    Diagnostic, GraphicalReportHandler, GraphicalTheme, LabeledSpan, ReportHandler, Severity,
    SourceCode,
};
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

pub fn diagnostic_to_string(
    diagnostic: &dyn Diagnostic,
    report_handler: &dyn ReportHandler,
) -> String {
    struct FmtHelper<'a> {
        fun: &'a dyn Fn(&mut Formatter<'_>) -> std::fmt::Result,
    }
    impl<'a> Debug for FmtHelper<'a> {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            (self.fun)(f)
        }
    }
    format!(
        "{:?}",
        FmtHelper {
            fun: &|f| report_handler.debug(diagnostic, f)
        }
    )
}

pub struct InterpreterDiagnostic {
    pub message: String,
    pub code: Option<String>,
    pub severity: Option<Severity>,
    pub help: Option<String>,
    pub labels: Vec<LabeledSpan>,
    pub source_file: SourceFileHandle,
}

impl InterpreterDiagnostic {
    #[track_caller]
    pub fn new(location: &Location, message: String) -> Self {
        let mut diagnostic =
            InterpreterDiagnostic::from_source_file(&location.source_file, message);
        diagnostic.add_primary_label(&location);
        diagnostic
    }

    #[track_caller]
    pub fn new_with(
        location: &Location,
        message: String,
        mut f: impl FnMut(&mut InterpreterDiagnostic),
    ) -> Self {
        let mut diagnostic = InterpreterDiagnostic::new(&location, message);
        f(&mut diagnostic);
        diagnostic
    }

    #[track_caller]
    pub fn from_source_file(source_file: &SourceFileHandle, message: String) -> Self {
        InterpreterDiagnostic {
            message,
            code: None,
            severity: None,
            help: None,
            labels: Vec::new(),
            source_file: source_file.clone(),
        }
    }

    pub fn add_primary_label(&mut self, location: &Location) {
        self.labels.push(LabeledSpan::new_primary_with_span(
            None,
            location.start_byte as usize..location.end_byte as usize,
        ));
    }

    pub fn add_label<S: Into<String>>(&mut self, location: &Location, message: S) {
        self.labels.push(LabeledSpan::at(
            location.start_byte as usize..location.end_byte as usize,
            message,
        ));
    }

    pub fn set_help<S: Into<String>>(&mut self, help: S) {
        self.help = Some(help.into());
    }
}

impl Error for InterpreterDiagnostic {}

impl Display for InterpreterDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.message)
    }
}

impl Debug for InterpreterDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.message)?;
        GraphicalReportHandler::new()
            .with_theme(GraphicalTheme::unicode_nocolor())
            .render_report(f, self)
    }
}

impl Diagnostic for InterpreterDiagnostic {
    fn code<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
        self.code
            .as_ref()
            .map(Box::new)
            .map(|c| c as Box<dyn Display>)
    }

    fn severity(&self) -> Option<Severity> {
        self.severity
    }

    fn help<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
        self.help
            .as_ref()
            .map(Box::new)
            .map(|c| c as Box<dyn Display>)
    }

    fn source_code(&self) -> Option<&dyn SourceCode> {
        Some(&self.source_file)
    }
    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        Some(Box::new(self.labels.clone().into_iter()))
    }
}
pub fn unwrap_diagnostic_to_string<T>(result: &FelicoResult<T>) -> String {
    let mut string = String::new();
    if let Err(stack) = &result {
        for frame in stack.report.frames() {
            if let Some(error) = frame.downcast_ref::<FelicoError>() {
                if let FelicoError::Diagnostic(diagnostic) = error {
                    string += &diagnostic_to_string(
                        diagnostic,
                        &GraphicalReportHandler::new()
                            .with_theme(GraphicalTheme::unicode_nocolor()),
                    )
                    .trim()
                    .to_string();
                    string += "\n\n";
                } else {
                    string += &format!("{:?}\n\n", error);
                }
            }
        }
    } else {
        panic!("Expected error result, but got Ok(...) instead");
    }
    string
}

#[cfg(test)]
pub fn assert_diagnostic<T>(result: &FelicoResult<T>, expected: expect_test::Expect) {
    expected.assert_eq(&unwrap_diagnostic_to_string(result));
}

#[cfg(test)]
mod tests {
    use crate::infra::diagnostic::{
        assert_diagnostic, diagnostic_to_string, InterpreterDiagnostic,
    };
    use crate::infra::location::Location;
    use crate::infra::result::{FelicoError, FelicoReport};
    use crate::infra::source_file::SourceFileHandle;
    use error_stack::Report;
    use expect_test::expect;
    use miette::{GraphicalReportHandler, GraphicalTheme, LabeledSpan};
    use std::io::ErrorKind;

    #[test]
    fn test_diagnostic_printing() {
        let diagnostic = InterpreterDiagnostic {
            message: "Something went wrong".to_string(),
            code: Some("code::foo::bar".to_string()),
            severity: None,
            help: Some("Helpful hint".to_string()),
            labels: vec![
                LabeledSpan::at(0..3, "This should be Rust"),
                LabeledSpan::new_primary_with_span(Some("Yay!".to_string()), 4..9),
            ],
            source_file: SourceFileHandle::from_string("foo.txt", "foo rocks!"),
        };

        let mut graphical_report_handler = GraphicalReportHandler::new();
        let should_print = std::env::var("PRINT_DIAGNOSTICS").is_ok();
        if should_print {
            println!(
                "{}",
                diagnostic_to_string(&diagnostic, &graphical_report_handler)
            );

            graphical_report_handler =
                graphical_report_handler.with_theme(GraphicalTheme::unicode());
            println!(
                "{}",
                diagnostic_to_string(&diagnostic, &graphical_report_handler)
            );

            graphical_report_handler =
                graphical_report_handler.with_theme(GraphicalTheme::unicode_nocolor());
            println!(
                "{}",
                diagnostic_to_string(&diagnostic, &graphical_report_handler)
            );
        }

        assert_diagnostic::<()>(
            &Err(FelicoReport::from(diagnostic)),
            expect![[r#"
                code::foo::bar

                  × Something went wrong
                   ╭─[foo.txt:1:5]
                 1 │ foo rocks!
                   · ─┬─ ──┬──
                   ·  │    ╰── Yay!
                   ·  ╰── This should be Rust
                   ╰────
                  help: Helpful hint

            "#]],
        );
    }

    #[test]
    fn test_diagnostic_printing_multiple_frames() {
        let mut report = Report::from(FelicoError::from(InterpreterDiagnostic::new(
            &Location::ephemeral(),
            "foo".to_string(),
        )));
        let error = FelicoError::from(std::io::Error::from(ErrorKind::AddrNotAvailable));
        let other_report = Report::from(error);
        report.extend_one(other_report);
        assert_diagnostic::<()>(
            &Err(FelicoReport { report }),
            expect![[r#"
            code::foo::bar

              × Something went wrong
               ╭─[foo.txt:1:5]
             1 │ foo rocks!
               · ─┬─ ──┬──
               ·  │    ╰── Yay!
               ·  ╰── This should be Rust
               ╰────
              help: Helpful hint"#]],
        );
    }
}
