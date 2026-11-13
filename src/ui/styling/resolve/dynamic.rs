//! Déclarations dépendant de propriétés personnalisées (feuilles de frameworks) :
//! évaluées dans la portée calculée de l'élément, à leur rang parmi les blocs actifs.
use super::environment::StyleEnvironment;
use crate::ui::styling::{Properties, Styles, Typography, tokens};

/// Ce qui entoure l'évaluation : typographie héritée, police racine et rayons du parent
/// (`border-*-radius: inherit`).
pub(super) struct Surroundings<'a> {
    pub(super) inherited: Typography,
    pub(super) root_font: f32,
    pub(super) parent_corners: [f32; 4],
    pub(super) env: &'a StyleEnvironment<'a>,
}

/// Propriétés de l'élément avec ses déclarations dynamiques évaluées : la base, puis les
/// blocs d'état actifs (`active`, indices dans `styles.states`), puis les règles du
/// parent pour ses enfants (`child_rules`) dessous. `local` est le résultat de la
/// cascade sans évaluation (taille de police et couleur de départ).
pub(super) fn evaluate(
    styles: &Styles,
    local: Properties,
    active: &[usize],
    child_rules: Option<Properties>,
    around: Surroundings<'_>,
) -> crate::Result<Properties> {
    let Surroundings {
        inherited,
        root_font,
        parent_corners,
        env,
    } = around;
    let black = crate::ui::theme::Color::rgb(0, 0, 0);
    let mut eval = tokens::Evaluation {
        variables: styles.variables.as_ref(),
        globals: styles.globals,
        font: local.text.size.unwrap_or(inherited.size.unwrap_or(16.)),
        parent_font: inherited.size.unwrap_or(16.),
        root_font,
        viewport: (env.viewport.width, env.viewport.height),
        scale: if styles.scale > 0. { styles.scale } else { 1. },
        current: local.text.color.or(inherited.color).unwrap_or(black),
        parent_corners,
    };
    let run = |eval: &tokens::Evaluation| {
        let mut out = styles.normal;
        if let Some(d) = out.dynamic {
            tokens::evaluate(d, eval, &mut out);
            if out.corner_edges.iter().any(Option::is_some) {
                // Coins isolés de la base : posés sur ses propres rayons.
                out = out.over(Properties::default());
            }
        }
        for &i in active {
            let mut block = styles.states[i].1;
            if let Some(d) = block.dynamic {
                tokens::evaluate(d, eval, &mut block);
            }
            out = block.over(out);
        }
        match child_rules {
            Some(rules) => out.over(rules),
            None => out,
        }
    };
    let mut local = run(&eval);
    // `em` suit la taille de police finale de l'élément.
    let font = local.text.size.unwrap_or(eval.parent_font);
    let current = local.text.color.or(inherited.color).unwrap_or(black);
    if font != eval.font || current != eval.current {
        eval.font = font;
        eval.current = current;
        local = run(&eval);
    }
    local.validate()?;
    Ok(local)
}
