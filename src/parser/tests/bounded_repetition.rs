use super::*;

#[test]
fn bounded_repetition_at_most_m() {
    assert_eq!(ast("a{,3}"), br(lit('a'), 0, Some(3), false));
    assert_eq!(ast("a{,33333}"), br(lit('a'), 0, Some(33333), false));
    assert_eq!(
        ast("a{                 ,              3             }"),
        br(lit('a'), 0, Some(3), false)
    );
    assert_eq!(
        ast("a{                 ,              3             }?"),
        br(lit('a'), 0, Some(3), true)
    );
    assert_eq!(
        ast("a{                 ,              3             }?b"),
        cat(br(lit('a'), 0, Some(3), true), lit('b'))
    );
    assert_eq!(
        ast("a{                 ,              3             } ?b"),
        cat(
            br(lit('a'), 0, Some(3), false),
            cat(question(lit(' ')), lit('b'))
        )
    );
}

#[test]
fn bounded_repetition_fake_at_most_m() {
    assert_eq!(
        ast("a{,a3}"),
        cat(
            lit('a'),
            cat(
                lit('{'),
                cat(lit(','), cat(lit('a'), cat(lit('3'), lit('}'))))
            )
        )
    );

    assert_eq!(
        ast("a{,3a}"),
        cat(
            lit('a'),
            cat(
                lit('{'),
                cat(lit(','), cat(lit('3'), cat(lit('a'), lit('}'))))
            )
        )
    );
}

#[test]
fn bounded_repetition_at_most_m_with_overflow() {
    let result = parse("a{,65536}");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err, ParserError::LimitExceeded("{,65536}".to_string()));

    let result = parse("a{          ,              65536}");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(
        err,
        ParserError::LimitExceeded("{          ,              65536}".to_string())
    );
}

#[test]
fn bounded_repetition_exactly_n() {
    assert_eq!(ast("a{3}"), br(lit('a'), 3, Some(3), false));
    assert_eq!(ast("a{33333}"), br(lit('a'), 33333, Some(33333), false));
    assert_eq!(
        ast("a{                 3             }"),
        br(lit('a'), 3, Some(3), false)
    );
    assert_eq!(
        ast("a{                 3             }?"),
        br(lit('a'), 3, Some(3), true)
    );
    assert_eq!(
        ast("a{                 3             }?b"),
        cat(br(lit('a'), 3, Some(3), true), lit('b'))
    );
    assert_eq!(
        ast("a{                 3             } ?b"),
        cat(
            br(lit('a'), 3, Some(3), false),
            cat(question(lit(' ')), lit('b'))
        )
    );
}

#[test]
fn bounded_repetition_exactly_n_with_overflow() {
    let result = parse("a{65536}");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err, ParserError::LimitExceeded("{65536}".to_string()));
    let result = parse("a{          65536}");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(
        err,
        ParserError::LimitExceeded("{          65536}".to_string())
    );
}

#[test]
fn bounded_repetition_fake_exactly_n() {
    assert_eq!(
        ast("a{3a}"),
        cat(
            lit('a'),
            cat(lit('{'), cat(lit('3'), cat(lit('a'), lit('}'))))
        )
    );
}

#[test]
fn bounded_repetition_at_least_n() {
    assert_eq!(ast("a{3,}"), br(lit('a'), 3, None, false));
    assert_eq!(ast("a{33333,}"), br(lit('a'), 33333, None, false));
    assert_eq!(
        ast("a{                 3             ,}"),
        br(lit('a'), 3, None, false)
    );
    assert_eq!(
        ast("a{                 3             ,}?"),
        br(lit('a'), 3, None, true)
    );
    assert_eq!(
        ast("a{                 3             ,}?b"),
        cat(br(lit('a'), 3, None, true), lit('b'))
    );
    assert_eq!(
        ast("a{                 3             ,} ?b"),
        cat(
            br(lit('a'), 3, None, false),
            cat(question(lit(' ')), lit('b'))
        )
    );
}

#[test]
fn bounded_repetition_at_least_n_with_overflow() {
    let result = parse("a{65536,}");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err, ParserError::LimitExceeded("{65536,}".to_string()));
    let result = parse("a{          65536,}");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(
        err,
        ParserError::LimitExceeded("{          65536,}".to_string())
    );
    let result = parse("a{          65536,     }");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(
        err,
        ParserError::LimitExceeded("{          65536,     }".to_string())
    );
}

#[test]
fn bounded_repetition_fake_at_least_n() {
    assert_eq!(
        ast("a{3,a}"),
        cat(
            lit('a'),
            cat(
                lit('{'),
                cat(lit('3'), cat(lit(','), cat(lit('a'), lit('}'))))
            )
        )
    );
}

#[test]
fn bounded_repetition_n_m() {
    assert_eq!(ast("a{3,5}"), br(lit('a'), 3, Some(5), false));
    assert_eq!(
        ast("a{33333,55555}"),
        br(lit('a'), 33333, Some(55555), false)
    );
    assert_eq!(
        ast("a{                 3             ,              5             }"),
        br(lit('a'), 3, Some(5), false)
    );
    assert_eq!(
        ast("a{                 3             ,              5             }?"),
        br(lit('a'), 3, Some(5), true)
    );
    assert_eq!(
        ast("a{                 3             ,              5             }?b"),
        cat(br(lit('a'), 3, Some(5), true), lit('b'))
    );
    assert_eq!(
        ast("a{                 3             ,              5             } ?b"),
        cat(
            br(lit('a'), 3, Some(5), false),
            cat(question(lit(' ')), lit('b'))
        )
    );
}

#[test]
fn bounded_repetition_n_greater_than_m() {
    let result = parse("a{3,2}");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err, ParserError::MinimumGreaterThanMaximum(3, 2));
}

#[test]
fn bounded_repetition_n_greater_than_m_with_overflow() {
    let result = parse("a{65536,65535}");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err, ParserError::LimitExceeded("{65536,65535}".to_string()));
    let result = parse("a{65535,65536}");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err, ParserError::LimitExceeded("{65535,65536}".to_string()));
    let result = parse("a{          65536,     65535}");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(
        err,
        ParserError::LimitExceeded("{          65536,     65535}".to_string())
    );
}

#[test]
fn bounded_repetition_fake_n_greater_than_m() {
    assert_eq!(
        ast("a{3,2a}"),
        cat(
            lit('a'),
            cat(
                lit('{'),
                cat(
                    lit('3'),
                    cat(lit(','), cat(lit('2'), cat(lit('a'), lit('}'))))
                )
            )
        )
    );
}

#[test]
fn bounded_repetition_fake_n_greater_than_m_with_overflow() {
    assert_eq!(
        ast("a{65536,65535a}"),
        cat(
            lit('a'),
            cat(
                lit('{'),
                cat(
                    lit('6'),
                    cat(
                        lit('5'),
                        cat(
                            lit('5'),
                            cat(
                                lit('3'),
                                cat(
                                    lit('6'),
                                    cat(
                                        lit(','),
                                        cat(
                                            lit('6'),
                                            cat(
                                                lit('5'),
                                                cat(
                                                    lit('5'),
                                                    cat(
                                                        lit('3'),
                                                        cat(lit('5'), cat(lit('a'), lit('}')))
                                                    )
                                                )
                                            )
                                        )
                                    )
                                )
                            )
                        )
                    )
                )
            )
        )
    );
}

// --- stacking with other quantifiers ----------------------------------------
//
// `{n,m}` is a quantifier like any other, so `parse_repetition`'s loop must
// let a following `*`/`+`/`{n,m}` wrap it, the same way it already lets `*`
// wrap `+` in `a+*`. Stacking is always language-redundant (same argument as
// `stacked_plus`/`repetition_operators_stack_left_to_right_when_mixed`) but
// syntactically legal -- `{n,m}` isn't special enough to be the one
// quantifier that can't be stacked onto.
//
// A trailing `?` is deliberately excluded from this section: it's already
// claimed by `{n,m}`'s own lazy-suffix parsing (see `bounded_repetition_n_m`,
// `"a{3,5}?"`), the same collision `a??` already resolved back in Lesson 5.a
// -- it never reaches this loop as a fresh quantifier to stack.

#[test]
fn bounded_repetition_can_be_stacked_with_other_quantifiers() {
    assert_eq!(ast("a{3,4}+"), plus(br(lit('a'), 3, Some(4), false)));
    assert_eq!(ast("a{3,4}*"), star(br(lit('a'), 3, Some(4), false)));
    // Order reversed: whatever `node` already is gets wrapped -- `{n,m}`
    // doesn't care that its child is already a `Plus`.
    assert_eq!(ast("a+{2,3}"), br(plus(lit('a')), 2, Some(3), false));
    // Two bonded repetitions stacked, same shape as `stacked_stars`.
    assert_eq!(
        ast("a{3,4}{2,5}"),
        br(br(lit('a'), 3, Some(4), false), 2, Some(5), false)
    );
}

#[test]
fn bounded_repetition_followed_by_double_plus_is_stacking_not_possessive() {
    // In PCRE, `X+` immediately after any quantifier is a *possessive*
    // suffix, not a second quantifier -- `a{3,4}+` means "possessive
    // {3,4}", and a second `+` after that has nothing left to attach to
    // (regex101 rejects `a{3,4}++`). This project deliberately doesn't
    // implement possessive quantifiers -- they require discarding a live
    // thread because of an earlier commitment, which breaks the "a thread's
    // fate depends only on (state, position)" invariant the whole simulation
    // relies on, the same reason backreferences are impossible. Since that
    // feature isn't implemented, `+` after a quantifier is never claimed by
    // it -- it's just `Plus` wrapping whatever came before, same as `a++`
    // already means `Plus(Plus(a))`. This is an intentional divergence from
    // PCRE syntax, not a bug to reconcile with it.
    assert_eq!(
        ast("a{3,4}++"),
        plus(plus(br(lit('a'), 3, Some(4), false)))
    );
}
