#version 450
// Couverture (boîte arrondie, anneau de trait, segment, ou texture) par distance au bord
// avec anticrénelage, multipliée par la découpe rectangulaire et par la chaîne des
// découpes arrondies et des masques en dégradé, puis peinture (couleur, dégradé,
// texture, masque teinté, dégradé vu à travers un masque de glyphes). Couleurs
// prémultipliées, mélange « source-over ». Port de `macos/metal/shaders.metal`.
#extension GL_GOOGLE_include_directive : require
#include "common.glsl"

layout(std430, set = 0, binding = 1) readonly buffer Stops { Stop stops[]; };
layout(std430, set = 0, binding = 2) readonly buffer Clips { ClipNode clips[]; };
layout(set = 0, binding = 3) uniform sampler2D image;

layout(location = 0) in vec2 local;
layout(location = 1) in vec2 uv;
layout(location = 2) flat in uint instance;
layout(location = 0) out vec4 result;

const uint SHAPE_BOX = 1u;
const uint SHAPE_RING = 2u;
const uint SHAPE_SEGMENT = 3u;
const uint PAINT_LINEAR = 1u;
const uint PAINT_RADIAL = 2u;
const uint PAINT_TEXTURE = 3u;
const uint PAINT_MASK = 4u;

// Part d'un pixel carré (côté `px`, centré en `p`) couverte par le rectangle `r`.
float box_cover(vec2 p, vec4 r, float px) {
    vec2 lo = max(p - 0.5 * px, r.xy);
    vec2 hi = min(p + 0.5 * px, r.zw);
    vec2 o = clamp((hi - lo) / px, 0.0, 1.0);
    return o.x * o.y;
}
// Couverture d'un rectangle à coins arrondis indépendants : aire exacte le long des
// côtés, distance à l'arc dans les coins.
float rounded_cover(vec2 p, vec4 r, vec4 radii, float px) {
    float cover = box_cover(p, r, px);
    if (cover <= 0.0) {
        return 0.0;
    }
    vec2 center = (r.xy + r.zw) * 0.5;
    vec2 q = p - center;
    float radius = q.x < 0.0 ? (q.y < 0.0 ? radii.x : radii.w) : (q.y < 0.0 ? radii.y : radii.z);
    if (radius > 0.0) {
        vec2 d = abs(q) - (r.zw - r.xy) * 0.5 + radius;
        if (d.x > 0.0 && d.y > 0.0) {
            float distance = length(d) - radius;
            cover = min(cover, clamp(0.5 - distance / px, 0.0, 1.0));
        }
    }
    return cover;
}
// Segment à bouts plats de demi-épaisseur `half_width`.
float segment_cover(vec2 p, vec4 s, float half_width, float px) {
    vec2 d = s.zw - s.xy;
    float len = length(d);
    if (len <= 0.0) {
        return 0.0;
    }
    vec2 u = d / len;
    vec2 n = vec2(-u.y, u.x);
    vec2 q = p - s.xy;
    float along = dot(q, u);
    float across = dot(q, n);
    float a = clamp((min(along + 0.5 * px, len) - max(along - 0.5 * px, 0.0)) / px, 0.0, 1.0);
    float c = clamp((min(across + 0.5 * px, half_width) - max(across - 0.5 * px, -half_width)) / px,
                    0.0, 1.0);
    return a * c;
}
// Couleur du dégradé en `t`, couleurs extrêmes prolongées (comme Cairo en EXTEND_PAD).
vec4 ramp(float t, uint first, uint count) {
    if (t <= stops[first].position.x) {
        return stops[first].color;
    }
    for (uint k = 1u; k < count; k++) {
        Stop b = stops[first + k];
        if (t <= b.position.x) {
            Stop a = stops[first + k - 1u];
            float w = b.position.x - a.position.x;
            float f = w > 0.0 ? (t - a.position.x) / w : 1.0;
            return mix(a.color, b.color, f);
        }
    }
    return stops[first + count - 1u].color;
}
float chain_cover(vec2 pixel, int index) {
    float cover = 1.0;
    for (int k = 0; index >= 0 && k < 64; k++) {
        ClipNode n = clips[index];
        vec2 p = vec2(pixel.x * n.m.x + pixel.y * n.m.z + n.t.x,
                      pixel.x * n.m.y + pixel.y * n.m.w + n.t.y);
        if (n.link.y == 1) {
            // Masque : alpha du dégradé, couleurs extrêmes prolongées.
            vec2 d = n.rect.zw - n.rect.xy;
            float len = dot(d, d);
            float t = len > 0.0 ? dot(p - n.rect.xy, d) / len : 0.0;
            cover *= ramp(t, uint(n.link.z), uint(n.link.w)).a;
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
vec4 premultiply(vec4 c) {
    return vec4(c.rgb * c.a, c.a);
}
void main() {
    Instance i = instances[instance];
    float px = i.t.z;
    float cover = 1.0;
    uint shape = i.kind.x;
    if (shape == SHAPE_BOX) {
        cover = rounded_cover(local, i.shape, i.radii, px);
    } else if (shape == SHAPE_RING) {
        cover = max(rounded_cover(local, i.shape, i.radii, px) -
                        rounded_cover(local, i.inner, i.iradii, px),
                    0.0);
    } else if (shape == SHAPE_SEGMENT) {
        cover = segment_cover(local, i.shape, i.radii.x, px);
    }
    vec2 pixel = gl_FragCoord.xy + frame.origin;
    cover *= box_cover(pixel, i.clip, 1.0);
    if (i.extra.x >= 0 && cover > 0.0) {
        cover *= chain_cover(pixel, i.extra.x);
    }
    if (cover <= 0.0) {
        // Hors de la forme ou de la découpe : ni peinture ni échantillonnage.
        result = vec4(0.0);
        return;
    }
    vec4 color;
    uint paint = i.kind.y;
    if (paint == PAINT_LINEAR) {
        vec2 a = i.paint.xy;
        vec2 d = i.paint.zw - a;
        float len = dot(d, d);
        float t = len > 0.0 ? dot(local - a, d) / len : 0.0;
        color = premultiply(ramp(t, i.kind.z, i.kind.w));
    } else if (paint == PAINT_RADIAL) {
        float t = length((local - i.paint.xy) / i.paint.zw);
        color = premultiply(ramp(t, i.kind.z, i.kind.w));
    } else if (paint == PAINT_TEXTURE) {
        color = texture(image, uv);
    } else if (paint == PAINT_MASK) {
        color = premultiply(i.color) * texture(image, uv).r;
    } else {
        color = premultiply(i.color);
    }
    if (i.extra.y == 1) {
        // Glyphe sous `background-clip: text` : la peinture passe par sa couverture.
        color *= texture(image, uv).r;
    }
    result = color * (cover * i.t.w);
}
