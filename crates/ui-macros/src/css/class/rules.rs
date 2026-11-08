//! Ajout des déclarations d'une règle à une classe : propriétés de base, états,
//! pseudo-éléments `::before`/`::after`, `::placeholder` et `::-webkit-scrollbar`.
use super::{Class, base_display, state_properties};
use crate::css::{animation, framework, parser, selector, values};
use std::collections::BTreeMap;
/// Ajoute les déclarations d'une règle à `class` ; l'erreur porte la position de la
/// déclaration fautive.
pub(in crate::css) fn add_rule(
    class: &mut Class,
    selected: &selector::Selector,
    conditional_media: bool,
    declarations: &[parser::Declaration],
    vars: &BTreeMap<String, String>,
    declaration_order: &mut u32,
) -> std::result::Result<(), (usize, String)> {
    let state = selected.state.as_str();
    if selected.pseudo == Some("scrollbar") {
        // `::-webkit-scrollbar { width: 8px }` : épaisseur de la gouttière de l'hôte,
        // comme WebView2 ; le pouce et la piste restent ceux de scrollbar-color.
        let mut size: Option<f32> = None;
        for d in declarations {
            *declaration_order += 1;
            let error = |m: String| (d.at, m);
            if selected.predicate.is_some() {
                return Err(error(
                    "::-webkit-scrollbar in a state, @media or @container rule is not implemented"
                        .into(),
                ));
            }
            if d.name != "width" && d.name != "height" {
                return Err(error(format!(
                    "::-webkit-scrollbar supports width and height only, not {}; use scrollbar-color for the thumb and track",
                    d.name
                )));
            }
            let value = values::resolve(&d.value, vars).map_err(error)?;
            let px = value
                .strip_suffix("px")
                .and_then(|v| v.trim().parse::<f32>().ok())
                .filter(|v| (0. ..=64.).contains(v))
                .ok_or_else(|| {
                    error(format!(
                        "::-webkit-scrollbar {} expects a length in px within 0..=64",
                        d.name
                    ))
                })?;
            if size.is_some_and(|s| s != px) {
                return Err(error(
                    "::-webkit-scrollbar width and height must be equal (one scrolling axis per container)"
                        .into(),
                ));
            }
            size = Some(px);
        }
        if let Some(px) = size {
            class
                .props
                .push(("scrollbar_size".into(), format!("{px:?}f32")));
        }
        return Ok(());
    }
    if selected.pseudo == Some("placeholder") {
        // `::placeholder` : couleur du texte indicatif d'un champ, propriété de l'hôte.
        for d in declarations {
            *declaration_order += 1;
            let error = |m: String| (d.at, m);
            if d.name != "color" {
                return Err(error(format!(
                    "::placeholder supports color only, not {}",
                    d.name
                )));
            }
            let value = values::resolve(&d.value, vars).map_err(error)?;
            let color = values::declaration("color", &value, vars)
                .map_err(error)?
                .into_iter()
                .find(|(name, _)| name == "color")
                .map(|(_, code)| code)
                .filter(|code| !code.starts_with("dynamic:"))
                .ok_or_else(|| error("::placeholder needs a constant colour".into()))?;
            let predicate = selected.predicate.clone().unwrap_or_else(|| {
                "__ui_root::ui::styling::StyleState::Where(&__ui_root::ui::styling::StyleState::All(&[]))".into()
            });
            class.state_props.push((
                predicate,
                vec![("placeholder_color".into(), color)],
                *declaration_order,
            ));
        }
        return Ok(());
    }
    if let Some(kind) = selected.pseudo {
        // L'élément généré est un enfant de l'hôte : le prédicat de l'hôte devient
        // une condition sur son parent.
        let predicate = selected
            .predicate
            .as_ref()
            .map(|p| format!("__ui_root::ui::styling::StyleState::Parent(&{p})"));
        let pseudo = class.pseudos.entry(kind).or_default();
        for d in declarations {
            *declaration_order += 1;
            let error = |m: String| (d.at, m);
            if d.name == "content" {
                if predicate.is_some() {
                    return Err(error(
                        "content in a state or media rule is not implemented".into(),
                    ));
                }
                pseudo.content = values::pseudo_content(&d.value).map_err(error)?;
                continue;
            }
            if d.name == "animation" || d.name.starts_with("animation-") {
                return Err(error(
                    "Animations of pseudo-elements are not implemented".into(),
                ));
            }
            let props = if d.name.starts_with("--") {
                vec![(
                    "custom_property".to_string(),
                    format!(
                        "({:?},{})",
                        d.name,
                        values::custom_value(&d.value).map_err(error)?
                    ),
                )]
            } else if d.name == "display" && predicate.is_none() {
                let value = values::resolve(&d.value, vars).map_err(error)?;
                match base_display(&value).map_err(error)? {
                    Some(model) => pseudo.display = Some(model),
                    None if value == "none" => pseudo.content = None,
                    None => {}
                }
                continue;
            } else {
                values::declaration(&d.name, &d.value, vars).map_err(error)?
            };
            match &predicate {
                Some(p) => {
                    state_properties(&props).map_err(error)?;
                    pseudo
                        .state_props
                        .push((p.clone(), props, *declaration_order));
                }
                None => pseudo.props.extend(props),
            }
        }
        return Ok(());
    }
    for d in declarations {
        *declaration_order += 1;
        let error = |m: String| (d.at, m);
        if d.name.starts_with("--") {
            // Propriété personnalisée déclarée par une classe ou un état.
            let value = values::custom_value(&d.value).map_err(error)?;
            let entry = (
                "custom_property".to_string(),
                format!("({:?},{value})", d.name),
            );
            if let Some(predicate) = &selected.predicate {
                class
                    .state_props
                    .push((predicate.clone(), vec![entry], *declaration_order));
            } else {
                class.props.push(entry);
            }
            continue;
        }
        let value = values::resolve(&d.value, vars).map_err(error)?;
        if d.name == "box-shadow" && value.to_ascii_lowercase().contains("currentcolor") {
            // Ombre en `currentColor` (anneau `ring-2` sans couleur) : évaluée à la
            // résolution des styles avec la couleur de texte de l'élément, comme les
            // déclarations dynamiques des feuilles de framework.
            let tokens = framework::values::tokenize(&value).map_err(error)?;
            let predicate = selected.predicate.clone().unwrap_or_else(|| {
                "__ui_root::ui::styling::StyleState::Where(&__ui_root::ui::styling::StyleState::All(&[]))".into()
            });
            class.state_props.push((
                predicate,
                vec![(
                    "dynamic_declarations".into(),
                    format!(
                        "&[__ui_root::ui::styling::tokens::DynamicDeclaration{{property:__ui_root::ui::styling::tokens::DynamicProperty::BoxShadow,value:{}}}]",
                        framework::values::token_slice(&tokens)
                    ),
                )],
                *declaration_order,
            ));
            continue;
        }
        if d.name == "animation" || d.name.starts_with("animation-") {
            if let Some(predicate) = &selected.predicate {
                // `.pause .piste { animation-play-state: paused }`, `.piste.inverse {
                // animation-direction: reverse }` : réglages remplacés pendant la règle ;
                // changer les étapes (`animation-name`, raccourci) selon l'état reste refusé.
                let props = vec![animation::state_override(&d.name, &value).map_err(error)?];
                state_properties(&props).map_err(error)?;
                class
                    .state_props
                    .push((predicate.clone(), props, *declaration_order));
                continue;
            }
            animation::declaration(&d.name, &value, &mut class.animation).map_err(error)?;
            continue;
        }
        if let Some(predicate) = &selected.predicate {
            if !["opacity", "transform"].contains(&d.name.as_str()) {
                let props = values::declaration(&d.name, &d.value, vars).map_err(error)?;
                state_properties(&props).map_err(error)?;
                class
                    .state_props
                    .push((predicate.clone(), props, *declaration_order));
                continue;
            }
            if conditional_media
                || selected.classes.len() != 1
                || !["hover", "active"].contains(&state)
            {
                // Règle d'état générale (combinateurs, focus, media…) : effet posé
                // à la résolution des styles et animé par la transition.
                let props = values::declaration(&d.name, &d.value, vars).map_err(error)?;
                class
                    .state_props
                    .push((predicate.clone(), props, *declaration_order));
                continue;
            }
            // Validate even a declaration replaced by a later rule.
            values::declaration(&d.name, &d.value, vars).map_err(error)?;
            let states = if state == "hover" {
                &mut class.hover
            } else {
                &mut class.active
            };
            let props = values::declaration(&d.name, &d.value, vars).map_err(error)?;
            states.insert(d.name.clone(), props[0].1.clone());
            continue;
        }
        if d.name == "display" {
            let model = base_display(&value).map_err(error)?;
            class.display_kind = Some(match model.as_deref() {
                Some("flex") => 1,
                Some(_) => 2,
                None => 0,
            });
            match model {
                Some(model) => class.display = Some(model),
                None if value == "none" => {
                    class.props.push(("hidden".into(), "true".into()));
                }
                None if value == "inline-block" => {
                    class.props.push(("inline_level".into(), "true".into()));
                }
                None => {}
            }
            continue;
        }
        let props = values::declaration(&d.name, &d.value, vars).map_err(error)?;
        class.props.extend(props);
    }
    Ok(())
}
