use crate::source_snippet::SourceSnippet;
use crate::source_span::SourceSpan;
use annotate_snippets::renderer::DecorStyle;
use annotate_snippets::{Annotation, AnnotationKind, Group, Level, Renderer, Snippet};

#[derive(Debug)]
pub enum SourceMessageLevel {
    Error,
    Warning,
    Info,
}

#[derive(Debug)]
pub struct SourceMessage {
    level: SourceMessageLevel,
    message: String,
    source_snippet: SourceSnippet,
    labels: Vec<SourceLabel>,
}

#[derive(Debug)]
pub struct SourceLabel {
    span: SourceSpan,
    label: String,
}

impl SourceLabel {
    pub fn new(span: SourceSpan, label: String) -> Self {
        Self { span, label }
    }

    pub fn span(&self) -> &SourceSpan {
        &self.span
    }

    pub fn label(&self) -> &str {
        &self.label
    }
}

impl SourceMessage {
    pub fn new(level: SourceMessageLevel, message: String, source_snippet: SourceSnippet) -> Self {
        Self {
            level,
            message,
            source_snippet,
            labels: vec![],
        }
    }

    pub fn error(message: String, source_snippet: SourceSnippet) -> Self {
        Self::new(SourceMessageLevel::Error, message, source_snippet)
    }

    pub fn warning(message: String, source_snippet: SourceSnippet) -> Self {
        Self::new(SourceMessageLevel::Warning, message, source_snippet)
    }

    pub fn info(message: String, source_snippet: SourceSnippet) -> Self {
        Self::new(SourceMessageLevel::Info, message, source_snippet)
    }

    pub fn add_label(&mut self, source_label: SourceLabel) {
        self.labels.push(source_label);
    }

    pub fn render(&self) -> String {
        let renderer = Renderer::styled().decor_style(DecorStyle::Unicode);
        renderer.render(&self.create_report())
    }

    fn create_report(&self) -> Vec<Group> {
        let mut snippet: Snippet<Annotation> =
            Snippet::source(self.source_snippet.source_snippet())
                .line_start(self.source_snippet.start_line())
                .path(self.source_snippet.file_path());
        for label in &self.labels {
            snippet = snippet.annotation(
                AnnotationKind::Primary
                    .span(label.span.start()..label.span.end())
                    .label(label.label.clone()),
            );
        }
        let report_level = match self.level {
            SourceMessageLevel::Error => Level::ERROR,
            SourceMessageLevel::Warning => Level::WARNING,
            SourceMessageLevel::Info => Level::INFO,
        };
        let main_group: Group = report_level
            .primary_title(self.message.clone())
            .element(snippet);
        let report = vec![main_group];
        report
    }
}

#[cfg(test)]
mod tests {
    use crate::source_message::{SourceLabel, SourceMessage};
    use crate::source_snippet::SourceSnippet;
    use crate::source_span::SourceSpan;
    use expect_test::expect;
    use felico_base::unansi;

    #[test]
    fn test_source_message() {
        let source_snippet = SourceSnippet::new(
            "hello_world.felico".to_string(),
            "fun foo {}".to_string(),
            19,
            4,
        );
        let mut source_message = SourceMessage::error("test message".to_string(), source_snippet);
        source_message.add_label(SourceLabel::new(
            SourceSpan::new(4, 7),
            "test label".to_string(),
        ));

        let rendered_message = source_message.render();
        println!("{}", rendered_message);
        expect![[r#"
            error: test message
               ╭▸ hello_world.felico:19:5
               │
            19 │ fun foo {}
               ╰╴    ━━━ test label"#]]
        .assert_eq(&unansi(&rendered_message));
    }
}
