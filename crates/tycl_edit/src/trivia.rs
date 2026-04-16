#[derive(Clone, Debug, PartialEq)]
pub enum Trivia {
    EmptyLine,
    CommentLine(String),
}
