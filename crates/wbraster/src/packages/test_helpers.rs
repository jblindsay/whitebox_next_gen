use std::collections::BTreeSet;

pub(crate) fn assert_expected_csv_tokens_present(
    env_var: &str,
    actual_tokens: impl IntoIterator<Item = String>,
    token_kind: &str,
) {
    let Ok(expected_csv) = std::env::var(env_var) else {
        return;
    };

    let actual_vec: Vec<String> = actual_tokens.into_iter().collect();
    let actual_upper: BTreeSet<String> = actual_vec.iter().map(|s| s.to_ascii_uppercase()).collect();

    for token in expected_csv
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        let token_upper = token.to_ascii_uppercase();
        assert!(
            actual_upper.contains(&token_upper),
            "expected {token_kind} '{token}' from {env_var} to be present; actual values: {actual_vec:?}"
        );
    }
}
