use crate::lexer::TokenKind;
use crate::lexer::lex;
use crate::parser::TokenQuery;

#[test]
fn test_token_query_prev_next_nontrivia_around_trivia_and_eof() {
    let src = "a //c\nb";
    let tokens = lex(src).tokens;
    let q = TokenQuery::new(&tokens);

    // token sequence: Ident(a), LineComment, Newline, Ident(b), Eof
    assert!(matches!(tokens[0].kind, TokenKind::Ident(_)));
    assert!(matches!(tokens[1].kind, TokenKind::LineComment(_)));
    assert!(matches!(tokens[2].kind, TokenKind::Newline));
    assert!(matches!(tokens[3].kind, TokenKind::Ident(_)));
    assert!(matches!(tokens[4].kind, TokenKind::Eof));

    // prev_nontrivia treats idx as a boundary; it skips trivia.
    assert_eq!(q.prev_nontrivia(0), None);
    assert_eq!(q.prev_nontrivia(1), Some(0));
    assert_eq!(q.prev_nontrivia(2), Some(0));
    assert_eq!(q.prev_nontrivia(3), Some(0));
    assert_eq!(q.prev_nontrivia(4), Some(3));

    // next_nontrivia skips trivia, and EOF is non-trivia.
    assert_eq!(q.next_nontrivia(0), Some(0));
    assert_eq!(q.next_nontrivia(1), Some(3));
    assert_eq!(q.next_nontrivia(2), Some(3));
    assert_eq!(q.next_nontrivia(3), Some(3));
    assert_eq!(q.next_nontrivia(4), Some(4));
    assert_eq!(q.next_nontrivia(5), None);
}
