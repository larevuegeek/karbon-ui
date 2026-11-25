//! Clé de peinture de la toile de graphe (KUI-056) : le graphe, l'état et la palette.
use super::{super::model::*, Painter};
use crate::ui::editor::paint_hash::PaintHash;

impl Painter {
    /// Empreinte de ce que la toile dessine (hors sa place, son thème et son état).
    pub(super) fn key(&self) -> u64 {
        let mut h = PaintHash::new(0x0067_7261_7068);
        let Graph {
            nodes,
            wires,
            comments,
            compatible,
            many_per_input,
        } = &self.graph;
        h.usize(nodes.len());
        for node in nodes {
            node_hash(&mut h, node);
        }
        h.usize(wires.len());
        for GraphWire { from, to } in wires {
            for PortRef { node, port } in [from, to] {
                h.u64(*node);
                h.usize(*port);
            }
        }
        h.usize(comments.len());
        for GraphComment {
            id,
            rect,
            text,
            color,
        } in comments
        {
            h.u64(*id);
            h.rect(*rect);
            h.str(text);
            h.option_color(*color);
        }
        h.usize(compatible.len());
        for (from, to) in compatible {
            h.u64(u64::from(*from) << 32 | u64::from(*to));
        }
        h.bool(*many_per_input);
        h.debug(&self.state);
        h.debug(&self.palette);
        h.finish()
    }
}

fn node_hash(h: &mut PaintHash, node: &GraphNode) {
    let GraphNode {
        id,
        title,
        subtitle,
        position,
        width,
        color,
        mark,
        inputs,
        outputs,
    } = node;
    h.u64(*id);
    h.str(title);
    h.bool(subtitle.is_some());
    h.str(subtitle.as_deref().unwrap_or_default());
    h.point(*position);
    h.f32(*width);
    h.option_color(*color);
    h.option_color(*mark);
    for ports in [inputs, outputs] {
        h.usize(ports.len());
        for GraphPort { name, kind, color } in ports {
            h.str(name);
            h.u64(u64::from(*kind));
            h.option_color(*color);
        }
    }
}
