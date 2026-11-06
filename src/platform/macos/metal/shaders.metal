// Shaders du renderer Metal de karbon-ui, compilés au lancement (newLibraryWithSource).
//
// Une seule paire de fonctions dessine tous les éléments : un quad par instance, dont le
// fragment calcule la couverture (boîte arrondie, anneau de trait, segment, ou texture)
// par distance au bord avec anticrénelage, la multiplie par la découpe rectangulaire et
// par la chaîne des découpes arrondies et des masques en dégradé, puis la peinture
// (couleur, dégradé, texture, masque teinté, dégradé vu à travers un masque de glyphes).
// Couleurs prémultipliées, mélange « source-over ».
#include <metal_stdlib>
using namespace metal;

// Miroir exact de `metal::instance::Instance` (13 × 16 octets).
struct Instance {
    float4 quad;   // quad dans le repère local (gauche, haut, droite, bas)
    float4 m;      // repère local → pixels : m11 m12 m21 m22
    float4 t;      // dx, dy, taille d'un pixel dans le repère local, opacité
    float4 shape;  // boîte extérieure, ou segment (x0 y0 x1 y1)
    float4 radii;  // rayons extérieurs (hg hd bd bg) ; segment : demi-épaisseur en x
    float4 inner;  // boîte intérieure d'un anneau (trait)
    float4 iradii; // rayons intérieurs
    float4 color;  // couleur non prémultipliée
    float4 paint;  // dégradé linéaire (x0 y0 x1 y1) ou radial (cx cy rx ry)
    float4 uv;     // coordonnées de texture aux coins du quad
    float4 clip;   // découpe rectangulaire en pixels de la cible
    uint4 kind;    // forme, peinture, premier arrêt, nombre d'arrêts
    int4 extra;    // maillon de découpe générale (-1 : aucun) ; y = 1 : peinture
                   // multipliée par la texture (glyphes sous `background-clip: text`)
};
struct Stop {
    float4 color;    // non prémultipliée
    float4 position; // x
};
// Découpe arrondie : m, t.xy = pixels → repère de la découpe ; t.z = taille d'un pixel.
// Masque en dégradé (`mask-image`) : rect = extrémités du dégradé dans ce repère.
struct ClipNode {
    float4 m;
    float4 t;
    float4 rect;
    float4 radii;
    int4 link; // x : maillon parent (-1 : fin) ; y : 0 découpe, 1 masque ; z, w : arrêts
};
struct Frame {
    float2 origin; // coin de la cible dans les pixels de la fenêtre
    float2 size;   // taille de la texture cible
};
struct Varying {
    float4 position [[position]];
    float2 local;
    float2 uv;
    uint instance [[flat]];
};

constant uint SHAPE_NONE = 0;
constant uint SHAPE_BOX = 1;
constant uint SHAPE_RING = 2;
constant uint SHAPE_SEGMENT = 3;
constant uint PAINT_SOLID = 0;
constant uint PAINT_LINEAR = 1;
constant uint PAINT_RADIAL = 2;
constant uint PAINT_TEXTURE = 3;
constant uint PAINT_MASK = 4;

vertex Varying quad_vertex(uint vid [[vertex_id]],
                           uint iid [[instance_id]],
                           const device Instance* instances [[buffer(0)]],
                           constant Frame& frame [[buffer(1)]]) {
    Instance i = instances[iid];
    float2 unit = float2(float(vid & 1), float(vid >> 1));
    float2 local = mix(i.quad.xy, i.quad.zw, unit);
    float2 pixel = float2(local.x * i.m.x + local.y * i.m.z + i.t.x,
                           local.x * i.m.y + local.y * i.m.w + i.t.y);
    float2 ndc = (pixel - frame.origin) / frame.size * float2(2.0, -2.0) + float2(-1.0, 1.0);
    Varying out;
    out.position = float4(ndc, 0.0, 1.0);
    out.local = local;
    out.uv = mix(i.uv.xy, i.uv.zw, unit);
    out.instance = iid;
    return out;
}

// Part d'un pixel carré (côté `px`, centré en `p`) couverte par le rectangle `r`.
static float box_cover(float2 p, float4 r, float px) {
    float2 lo = max(p - 0.5 * px, r.xy);
    float2 hi = min(p + 0.5 * px, r.zw);
    float2 o = clamp((hi - lo) / px, 0.0, 1.0);
    return o.x * o.y;
}
// Couverture d'un rectangle à coins arrondis indépendants : aire exacte le long des
// côtés, distance à l'arc dans les coins.
static float rounded_cover(float2 p, float4 r, float4 radii, float px) {
    float cover = box_cover(p, r, px);
    if (cover <= 0.0) {
        return 0.0;
    }
    float2 center = (r.xy + r.zw) * 0.5;
    float2 q = p - center;
    float radius = q.x < 0.0 ? (q.y < 0.0 ? radii.x : radii.w) : (q.y < 0.0 ? radii.y : radii.z);
    if (radius > 0.0) {
        float2 d = abs(q) - (r.zw - r.xy) * 0.5 + radius;
        if (d.x > 0.0 && d.y > 0.0) {
            float distance = length(d) - radius;
            cover = min(cover, clamp(0.5 - distance / px, 0.0, 1.0));
        }
    }
    return cover;
}
// Segment à bouts plats de demi-épaisseur `half_width`.
static float segment_cover(float2 p, float4 s, float half_width, float px) {
    float2 d = s.zw - s.xy;
    float len = length(d);
    if (len <= 0.0) {
        return 0.0;
    }
    float2 u = d / len;
    float2 n = float2(-u.y, u.x);
    float2 q = p - s.xy;
    float along = dot(q, u);
    float across = dot(q, n);
    float a = clamp((min(along + 0.5 * px, len) - max(along - 0.5 * px, 0.0)) / px, 0.0, 1.0);
    float c = clamp((min(across + 0.5 * px, half_width) - max(across - 0.5 * px, -half_width)) / px,
                    0.0, 1.0);
    return a * c;
}
static float4 ramp(float t, uint first, uint count, const device Stop* stops);
static float chain_cover(float2 pixel, int index, const device ClipNode* nodes,
                         const device Stop* stops) {
    float cover = 1.0;
    for (int k = 0; index >= 0 && k < 64; k++) {
        ClipNode n = nodes[index];
        float2 p = float2(pixel.x * n.m.x + pixel.y * n.m.z + n.t.x,
                          pixel.x * n.m.y + pixel.y * n.m.w + n.t.y);
        if (n.link.y == 1) {
            // Masque : alpha du dégradé, couleurs extrêmes prolongées.
            float2 d = n.rect.zw - n.rect.xy;
            float len = dot(d, d);
            float t = len > 0.0 ? dot(p - n.rect.xy, d) / len : 0.0;
            cover *= ramp(t, uint(n.link.z), uint(n.link.w), stops).a;
        } else {
            cover *= rounded_cover(p, n.rect, n.radii, n.t.z);
        }
        if (cover <= 0.0) {
            return 0.0;
        }
        index = n.link.x;
    }
    return cover;
}
// Couleur du dégradé en `t`, couleurs extrêmes prolongées (comme Core Graphics).
static float4 ramp(float t, uint first, uint count, const device Stop* stops) {
    if (t <= stops[first].position.x) {
        return stops[first].color;
    }
    for (uint k = 1; k < count; k++) {
        Stop b = stops[first + k];
        if (t <= b.position.x) {
            Stop a = stops[first + k - 1];
            float w = b.position.x - a.position.x;
            float f = w > 0.0 ? (t - a.position.x) / w : 1.0;
            return mix(a.color, b.color, f);
        }
    }
    return stops[first + count - 1].color;
}
static float4 premultiply(float4 c) {
    return float4(c.rgb * c.a, c.a);
}

fragment float4 quad_fragment(Varying in [[stage_in]],
                              const device Instance* instances [[buffer(0)]],
                              const device Stop* stops [[buffer(1)]],
                              const device ClipNode* clips [[buffer(2)]],
                              constant Frame& frame [[buffer(3)]],
                              texture2d<float> image [[texture(0)]]) {
    constexpr sampler smooth(coord::normalized, address::clamp_to_edge, filter::linear,
                             mip_filter::linear);
    Instance i = instances[in.instance];
    float px = i.t.z;
    float cover = 1.0;
    uint shape = i.kind.x;
    if (shape == SHAPE_BOX) {
        cover = rounded_cover(in.local, i.shape, i.radii, px);
    } else if (shape == SHAPE_RING) {
        cover = max(rounded_cover(in.local, i.shape, i.radii, px) -
                        rounded_cover(in.local, i.inner, i.iradii, px),
                    0.0);
    } else if (shape == SHAPE_SEGMENT) {
        cover = segment_cover(in.local, i.shape, i.radii.x, px);
    }
    float2 pixel = in.position.xy + frame.origin;
    cover *= box_cover(pixel, i.clip, 1.0);
    if (i.extra.x >= 0 && cover > 0.0) {
        cover *= chain_cover(pixel, i.extra.x, clips, stops);
    }
    if (cover <= 0.0) {
        // Hors de la forme ou de la découpe : ni peinture ni échantillonnage.
        return float4(0.0);
    }
    float4 color;
    uint paint = i.kind.y;
    if (paint == PAINT_LINEAR) {
        float2 a = i.paint.xy;
        float2 d = i.paint.zw - a;
        float len = dot(d, d);
        float t = len > 0.0 ? dot(in.local - a, d) / len : 0.0;
        color = premultiply(ramp(t, i.kind.z, i.kind.w, stops));
    } else if (paint == PAINT_RADIAL) {
        float t = length((in.local - i.paint.xy) / i.paint.zw);
        color = premultiply(ramp(t, i.kind.z, i.kind.w, stops));
    } else if (paint == PAINT_TEXTURE) {
        color = image.sample(smooth, in.uv);
    } else if (paint == PAINT_MASK) {
        color = premultiply(i.color) * image.sample(smooth, in.uv).r;
    } else {
        color = premultiply(i.color);
    }
    if (i.extra.y == 1) {
        // Glyphe sous `background-clip: text` : la peinture passe par sa couverture.
        color *= image.sample(smooth, in.uv).r;
    }
    return color * (cover * i.t.w);
}
