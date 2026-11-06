#version 450
// Passages de filtre du renderer Vulkan (`filter/`), miroir des filtres CPU de
// `platform/shared/filters.rs` (chemin Cairo) : réduction par moyenne de blocs, flou en
// boîte (trois boîtes par axe approchent la gaussienne, comme le chemin CPU),
// agrandissement bilinéaire, matrices de couleur enchaînées, ombre portée (alpha décalé
// teinté, puis fusion sous le contenu). Chaque passage couvre la zone utile de sa cible
// (RGBA 16 bits flottants) et lit sa source pixel par pixel ; hors de la zone utile de
// la source, tout est transparent, comme les bords des plans du chemin CPU.

// Miroir exact de `vulkan::filter::Params` (std430 : tableau de vec4).
struct FilterParams {
    vec4 size;      // largeur et hauteur utiles de la source, facteur k, rayon ou écart
    vec4 shift;     // décalage de l'ombre (x, y), nombre de matrices, direction (0 : x)
    vec4 tint;      // couleur de l'ombre, non prémultipliée
    vec4 rows[80];  // 16 matrices de 5 lignes (entrées r, g, b, a, décalage)
};
layout(std430, set = 0, binding = 0) readonly buffer Params {
    FilterParams params[];
};
layout(set = 0, binding = 1) uniform sampler2D source;
layout(set = 0, binding = 2) uniform sampler2D below;
layout(push_constant) uniform Push {
    uint index;
    uint kernel;
} push;
layout(location = 0) out vec4 result;

const uint DOWNSAMPLE = 0u;
const uint BOX = 1u;
const uint UPSAMPLE = 2u;
const uint MATRIX = 3u;
const uint SHADE = 4u;
const uint MERGE = 5u;

vec4 fetch(sampler2D image, ivec2 p, vec2 size) {
    if (p.x < 0 || p.y < 0 || p.x >= int(size.x) || p.y >= int(size.y)) {
        return vec4(0.0);
    }
    return texelFetch(image, p, 0);
}
// Moyenne du bloc k × k (pixels de la zone utile seulement, comme `downsample`).
vec4 downsample(FilterParams f, ivec2 p) {
    int k = int(f.size.z);
    ivec2 start = p * k;
    ivec2 end = min(start + k, ivec2(f.size.xy));
    vec4 sum = vec4(0.0);
    for (int y = start.y; y < end.y; y++) {
        for (int x = start.x; x < end.x; x++) {
            sum += texelFetch(source, ivec2(x, y), 0);
        }
    }
    int count = max((end.x - start.x) * (end.y - start.y), 1);
    return sum / float(count);
}
// Flou en boîte de rayon `size.w` dans la direction `shift.w` (0 : x, 1 : y) ;
// l'extérieur compte pour transparent (`box_horizontal`, `box_vertical`).
vec4 box(FilterParams f, ivec2 p) {
    int radius = int(f.size.w);
    ivec2 step = f.shift.w > 0.5 ? ivec2(0, 1) : ivec2(1, 0);
    vec4 sum = vec4(0.0);
    for (int i = -radius; i <= radius; i++) {
        sum += fetch(source, p + i * step, f.size.xy);
    }
    return sum / float(2 * radius + 1);
}
// Agrandissement bilinéaire d'un facteur k (même échantillonnage que `upsample`).
vec4 upsample(FilterParams f, ivec2 p) {
    float k = f.size.z;
    ivec2 n = max(ivec2(f.size.xy), ivec2(1));
    vec2 s = max((vec2(p) + 0.5) / k - 0.5, vec2(0.0));
    ivec2 base = min(ivec2(floor(s)), n - 1);
    vec2 fr = s - vec2(base);
    ivec2 next = min(base + 1, n - 1);
    vec4 a = texelFetch(source, base, 0);
    vec4 b = texelFetch(source, ivec2(next.x, base.y), 0);
    vec4 c = texelFetch(source, ivec2(base.x, next.y), 0);
    vec4 d = texelFetch(source, next, 0);
    vec4 top = a + (b - a) * fr.x;
    vec4 bottom = c + (d - c) * fr.x;
    return top + (bottom - top) * fr.y;
}
// Matrices de couleur sur les couleurs non prémultipliées, bornées après chacune ; un
// pixel transparent reste transparent sauf si la matrice ajoute de l'alpha
// (`color_matrix`).
vec4 matrices(FilterParams f, ivec2 p) {
    vec4 c = fetch(source, p, f.size.xy);
    int count = int(f.shift.z);
    for (int m = 0; m < count; m++) {
        vec4 offset = f.rows[m * 5 + 4];
        if (c.a <= 0.0 && offset.a <= 0.0) {
            continue;
        }
        vec4 s = c.a > 0.0 ? vec4(c.rgb / c.a, c.a) : vec4(0.0);
        s = clamp(s.r * f.rows[m * 5] + s.g * f.rows[m * 5 + 1] + s.b * f.rows[m * 5 + 2]
                + s.a * f.rows[m * 5 + 3] + offset, 0.0, 1.0);
        c = vec4(s.rgb * s.a, s.a);
    }
    return c;
}
void main() {
    FilterParams f = params[push.index];
    ivec2 p = ivec2(gl_FragCoord.xy);
    if (push.kernel == DOWNSAMPLE) {
        result = downsample(f, p);
    } else if (push.kernel == BOX) {
        result = box(f, p);
    } else if (push.kernel == UPSAMPLE) {
        result = upsample(f, p);
    } else if (push.kernel == MATRIX) {
        result = matrices(f, p);
    } else if (push.kernel == SHADE) {
        // Ombre : alpha de la source décalé d'un nombre entier de pixels, teinté.
        float a = fetch(source, p - ivec2(f.shift.xy), f.size.xy).a * f.tint.a;
        result = vec4(f.tint.rgb * a, a);
    } else {
        // Contenu (source) posé sur son ombre floutée (`below`).
        vec4 top = fetch(source, p, f.size.xy);
        result = top + fetch(below, p, f.size.xy) * (1.0 - top.a);
    }
}
