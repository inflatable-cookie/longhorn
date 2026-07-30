use longhorn_command::{CommandSurface, CommandVisibility};

use super::support::{command, keyword, minimal_registry};

#[test]
fn search_ranking_is_stable_across_label_keyword_category_id_and_description() {
    let exact = command("test:exact", "Open", "global");
    let prefix = command("test:prefix", "Open Project", "global");
    let mut keyword_match = command("test:keyword", "Load", "global");
    keyword_match.keywords = vec![keyword("open")];
    let mut description = command("test:description", "Browse", "global");
    description.description = Some("Open an existing project".to_owned());
    let registry = minimal_registry(vec![description, keyword_match, prefix, exact]);

    let hits = registry
        .search(CommandSurface::Palette, "open")
        .expect("search");
    assert_eq!(
        hits.iter()
            .map(|hit| hit.record.id.as_str())
            .collect::<Vec<_>>(),
        [
            "test:exact",
            "test:prefix",
            "test:keyword",
            "test:description"
        ]
    );
    assert_eq!(
        hits.iter().map(|hit| hit.score).collect::<Vec<_>>(),
        [0, 10, 30, 80]
    );
}

#[test]
fn search_uses_all_terms_and_stable_label_then_id_ties() {
    let mut first = command("test:b", "Same", "global");
    first.keywords = vec![keyword("project"), keyword("open")];
    let mut second = command("test:a", "Same", "global");
    second.keywords = vec![keyword("open"), keyword("project")];
    let registry = minimal_registry(vec![first, second]);

    let hits = registry
        .search(CommandSurface::Palette, "open project")
        .expect("search");
    assert_eq!(
        hits.iter()
            .map(|hit| hit.record.id.as_str())
            .collect::<Vec<_>>(),
        ["test:a", "test:b"]
    );
}

#[test]
fn surface_projection_and_hidden_visibility_are_explicit() {
    let mut palette = command("test:palette", "Palette", "global");
    palette.visibility = CommandVisibility {
        palette: true,
        ..CommandVisibility::default()
    };
    let mut menu = command("test:menu", "Menu", "global");
    menu.visibility = CommandVisibility {
        menu: true,
        ..CommandVisibility::default()
    };
    let mut hidden = command("test:hidden", "Hidden", "global");
    hidden.visibility = CommandVisibility::HIDDEN;
    let registry = minimal_registry(vec![hidden, menu, palette]);

    assert_eq!(registry.discovery(CommandSurface::Palette).count(), 1);
    assert_eq!(registry.discovery(CommandSurface::Menu).count(), 1);
    assert!(
        registry
            .search(CommandSurface::Palette, "hidden")
            .expect("search")
            .is_empty()
    );
}

#[test]
fn search_query_is_bounded() {
    let registry = minimal_registry(vec![command("test:one", "One", "global")]);
    let query = "x".repeat(registry.limits().maximum_search_query_bytes + 1);
    let error = registry
        .search(CommandSurface::Palette, &query)
        .expect_err("query must fail");
    assert_eq!(error.actual_bytes(), query.len());
}
