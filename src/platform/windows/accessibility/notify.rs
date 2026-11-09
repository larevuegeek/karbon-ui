//! Notifications UI Automation : différences entre deux snapshots traduites en
//! événements (menus, info-bulles, structure, focus) et en propriétés modifiées.
use super::*;
use windows::Win32::UI::Accessibility::{
    StructureChangeType, UIA_EVENT_ID, UiaRaiseAutomationEvent,
    UiaRaiseAutomationPropertyChangedEvent, UiaRaiseStructureChangedEvent,
};

pub(super) const MENU_OPENED_EVENT_20003: i32 = 20003;
pub(super) const MENU_CLOSED_EVENT_20007: i32 = 20007;
const TOOLTIP_OPENED_EVENT_20000: i32 = 20000;
const TOOLTIP_CLOSED_EVENT_20001: i32 = 20001;
pub(super) fn menu_events(old: &Snapshot, new: &Snapshot) -> Vec<(NodeId, i32)> {
    transient_events(
        old,
        new,
        50009,
        MENU_OPENED_EVENT_20003,
        MENU_CLOSED_EVENT_20007,
    )
}
fn transient_events(
    old: &Snapshot,
    new: &Snapshot,
    control_type: i32,
    opened: i32,
    closed: i32,
) -> Vec<(NodeId, i32)> {
    let menus = |snapshot: &Snapshot| {
        snapshot
            .nodes
            .iter()
            .filter(|n| n.control_type == control_type)
            .map(|n| n.key)
            .collect::<Vec<_>>()
    };
    let before = menus(old);
    let after = menus(new);
    before
        .iter()
        .filter(|key| !after.contains(key))
        .map(|key| (*key, closed))
        .chain(
            after
                .iter()
                .filter(|key| !before.contains(key))
                .map(|key| (*key, opened)),
        )
        .collect()
}
/// Émet un événement UIA ; un échec (aucun client) n'a pas d'effet sur l'état.
pub(super) fn raise(provider: &IRawElementProviderSimple, event: i32) {
    // SAFETY: fournisseur vivant ; UIA ne conserve que ses propres références.
    let _ = unsafe { UiaRaiseAutomationEvent(provider, UIA_EVENT_ID(event)) };
}
pub(super) fn raise_events(bridge: &Arc<Bridge>, old: &Snapshot, new: &Snapshot) {
    for (key, event) in menu_events(old, new).into_iter().chain(transient_events(
        old,
        new,
        50022,
        TOOLTIP_OPENED_EVENT_20000,
        TOOLTIP_CLOSED_EVENT_20001,
    )) {
        let provider = if event == MENU_CLOSED_EVENT_20007 || event == TOOLTIP_CLOSED_EVENT_20001 {
            old.node(key).map(|node| {
                let cached = lock(&bridge.providers)
                    .get(&key)
                    .and_then(|c| c.provider.upgrade());
                cached.unwrap_or_else(|| provider::create(bridge.clone(), &node))
            })
        } else {
            bridge.interface::<IRawElementProviderSimple>(key)
        };
        if let Ok(provider) = provider {
            raise(&provider, event);
        }
    }
    if old
        .nodes
        .iter()
        .map(|n| (n.key, n.parent))
        .ne(new.nodes.iter().map(|n| (n.key, n.parent)))
        && let Ok(provider) = bridge.interface::<IRawElementProviderSimple>(NodeId::Root)
    {
        // SAFETY: fournisseur vivant ; aucun identifiant d'exécution n'est transmis.
        let _ = unsafe {
            UiaRaiseStructureChangedEvent(&provider, StructureChangeType(2), null_mut(), 0)
        };
    }
    let previous = old.nodes.iter().find(|n| n.focused).map(|n| n.key);
    if let Some(node) = new.nodes.iter().find(|n| n.focused)
        && previous != Some(node.key)
        && let Ok(provider) = bridge.interface::<IRawElementProviderSimple>(node.key)
    {
        raise(&provider, 20005);
    }
    // Index des anciens nœuds : une recherche linéaire par nœud coûtait n² comparaisons
    // à chaque publication (des milliers de nœuds sur une liste de morceaux).
    let previous: std::collections::HashMap<NodeId, &Node> =
        old.nodes.iter().map(|n| (n.key, &**n)).collect();
    for node in &new.nodes {
        let Some(&before) = previous.get(&node.key) else {
            continue;
        };
        // Le fournisseur n'est demandé (créé au besoin) que pour un nœud qui change.
        if !changed(before, node) {
            continue;
        }
        let Ok(provider) = bridge.interface::<IRawElementProviderSimple>(node.key) else {
            continue;
        };
        if before.name != node.name {
            raise_property(
                &provider,
                30005,
                Variant::string(&before.name),
                Variant::string(&node.name),
            );
        }
        if before.help != node.help {
            raise_property(
                &provider,
                30013,
                Variant::string(&before.help),
                Variant::string(&node.help),
            );
        }
        // Position : seulement pour l'élément focalisé (loupe, suivi du focus). Comme dans
        // Chromium, un défilement ne signale ni le déplacement de chaque élément ni son
        // passage hors de la vue (`IsOffscreen`, lu à la demande) : des milliers
        // d'événements par image occupaient le thread de l'interface.
        if node.focused && before.bounds != node.bounds {
            raise_property(
                &provider,
                30001,
                Variant::rectangle(before.bounds),
                Variant::rectangle(node.bounds),
            );
        }
        for (property, a, b) in [
            (30174, before.is_dialog, node.is_dialog),
            (30010, before.enabled, node.enabled),
            (30009, before.focusable, node.focusable),
            (30008, before.focused, node.focused),
            (30046, before.read_only, node.read_only),
            (30031, before.invokable, node.invokable),
            (30043, before.value.is_some(), node.value.is_some()),
            (30040, before.text.is_some(), node.text.is_some()),
        ] {
            if a != b {
                raise_property(
                    &provider,
                    property,
                    Ok(Variant::boolean(a)),
                    Ok(Variant::boolean(b)),
                );
            }
        }
        if let (Some(before), Some(after)) = (&before.value, &node.value)
            && before != after
        {
            raise_property(
                &provider,
                30045,
                Variant::string(before),
                Variant::string(after),
            );
        }
        if let (Some(before), Some(after)) = (&before.text, &node.text) {
            if before.value != after.value {
                raise(&provider, 20015);
            }
            if before.selection != after.selection {
                raise(&provider, 20014);
            }
        }
        choices::raise_events(bridge, &provider, before, node, old, new);
        range_value::raise_events(&provider, before.range, node.range);
        expand::raise_events(&provider, before.expanded, node.expanded);
    }
}
/// Un événement de propriété est à émettre pour ce nœud (mêmes comparaisons que
/// [`raise_events`]).
fn changed(before: &Node, node: &Node) -> bool {
    before.name != node.name
        || before.help != node.help
        || (node.focused && before.bounds != node.bounds)
        || before.is_dialog != node.is_dialog
        || before.enabled != node.enabled
        || before.focusable != node.focusable
        || before.focused != node.focused
        || before.read_only != node.read_only
        || before.invokable != node.invokable
        || before.value != node.value
        || before.text.is_some() != node.text.is_some()
        || before
            .text
            .as_ref()
            .zip(node.text.as_ref())
            .is_some_and(|(a, b)| a.value != b.value || a.selection != b.selection)
        || before.choice != node.choice
        || node.selection_container()
        || before.range != node.range
        || before.expanded != node.expanded
}
/// Les deux valeurs sont libérées (VariantClear) au drop, même si l'émission échoue.
pub(super) fn raise_property(
    provider: &IRawElementProviderSimple,
    property: i32,
    old: Result<Variant>,
    new: Result<Variant>,
) {
    if let (Ok(old), Ok(new)) = (&old, &new) {
        // SAFETY: VARIANT valides copiés par UIA ; leur propriété reste ici.
        let _ = unsafe {
            UiaRaiseAutomationPropertyChangedEvent(
                provider,
                UIA_PROPERTY_ID(property),
                old.get(),
                new.get(),
            )
        };
    }
}
