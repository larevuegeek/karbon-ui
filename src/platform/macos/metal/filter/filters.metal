// Passages de filtre du renderer Metal (`filter/`), miroir de `graphics/filters.rs` :
// réduction par moyenne de blocs, flou gaussien séparable, agrandissement bilinéaire,
// matrices de couleur enchaînées, ombre portée (alpha décalé teinté, puis fusion sous le
// contenu). Chaque passage dessine un quad couvrant sa cible (RGBA 16 bits flottants)
// et lit sa source pixel par pixel ; hors de la zone utile de la source, tout est
// transparent, comme les bords des plans du chemin Core Graphics.
#include <metal_stdlib>
using namespace metal;

// Miroir exact de `metal::filter::Params`.
struct FilterParams {
    float4 size;      // largeur et hauteur utiles de la source, facteur k, écart type
    float4 shift;     // décalage de l'ombre (x, y), nombre de matrices, direction du flou
    float4 tint;      // couleur de l'ombre, non prémultipliée
    float4 rows[80];  // 16 matrices de 5 lignes (entrées r, g, b, a, décalage)
};
struct FilterVarying {
    float4 position [[position]];
};

vertex FilterVarying filter_vertex(uint vid [[vertex_id]]) {
    float2 unit = float2(float(vid & 1), float(vid >> 1));
    FilterVarying out;
    out.position = float4(unit.x * 2.0 - 1.0, 1.0 - unit.y * 2.0, 0.0, 1.0);
    return out;
}
static float4 fetch(texture2d<float> source, int2 p, float2 size) {
    if (p.x < 0 || p.y < 0 || p.x >= int(size.x) || p.y >= int(size.y)) {
        return float4(0.0);
    }
    return source.read(uint2(p));
}
// Moyenne du bloc k × k (pixels de la zone utile seulement, comme `downsample`).
fragment float4 filter_downsample(FilterVarying in [[stage_in]],
                                  constant FilterParams& f [[buffer(0)]],
                                  texture2d<float> source [[texture(0)]]) {
    int k = int(f.size.z);
    int2 start = int2(in.position.xy) * k;
    int2 end = min(start + k, int2(f.size.xy));
    float4 sum = float4(0.0);
    for (int y = start.y; y < end.y; y++) {
        for (int x = start.x; x < end.x; x++) {
            sum += source.read(uint2(x, y));
        }
    }
    int count = max((end.x - start.x) * (end.y - start.y), 1);
    return sum / float(count);
}
// Flou gaussien d'écart type `size.w` dans la direction `shift.w` (0 : x, 1 : y),
// noyau tronqué à 3 écarts types et normalisé ; l'extérieur compte pour transparent.
fragment float4 filter_blur(FilterVarying in [[stage_in]],
                            constant FilterParams& f [[buffer(0)]],
                            texture2d<float> source [[texture(0)]]) {
    float sigma = f.size.w;
    int radius = int(ceil(3.0 * sigma));
    int2 p = int2(in.position.xy);
    int2 step = f.shift.w > 0.5 ? int2(0, 1) : int2(1, 0);
    float scale = -0.5 / (sigma * sigma);
    float4 sum = fetch(source, p, f.size.xy);
    float total = 1.0;
    for (int i = 1; i <= radius; i++) {
        float w = exp(float(i * i) * scale);
        sum += w * (fetch(source, p + i * step, f.size.xy) + fetch(source, p - i * step, f.size.xy));
        total += 2.0 * w;
    }
    return sum / total;
}
// Agrandissement bilinéaire d'un facteur k (même échantillonnage que `upsample`).
fragment float4 filter_upsample(FilterVarying in [[stage_in]],
                                constant FilterParams& f [[buffer(0)]],
                                texture2d<float> source [[texture(0)]]) {
    float k = f.size.z;
    int2 n = max(int2(f.size.xy), int2(1));
    float2 s = max((floor(in.position.xy) + 0.5) / k - 0.5, float2(0.0));
    int2 base = min(int2(floor(s)), n - 1);
    float2 fr = s - float2(base);
    int2 next = min(base + 1, n - 1);
    float4 a = source.read(uint2(base));
    float4 b = source.read(uint2(next.x, base.y));
    float4 c = source.read(uint2(base.x, next.y));
    float4 d = source.read(uint2(next));
    float4 top = a + (b - a) * fr.x;
    float4 bottom = c + (d - c) * fr.x;
    return top + (bottom - top) * fr.y;
}
// Matrices de couleur sur les couleurs non prémultipliées, bornées après chacune.
fragment float4 filter_matrix(FilterVarying in [[stage_in]],
                              constant FilterParams& f [[buffer(0)]],
                              texture2d<float> source [[texture(0)]]) {
    float4 c = fetch(source, int2(in.position.xy), f.size.xy);
    int count = int(f.shift.z);
    if (count == 0) {
        return c;
    }
    if (c.a <= 0.0) {
        return float4(0.0);
    }
    float4 s = float4(c.rgb / c.a, c.a);
    for (int m = 0; m < count; m++) {
        constant float4* r = &f.rows[m * 5];
        s = clamp(s.r * r[0] + s.g * r[1] + s.b * r[2] + s.a * r[3] + r[4], 0.0, 1.0);
        if (s.a <= 0.0) {
            return float4(0.0);
        }
    }
    return float4(s.rgb * s.a, s.a);
}
// Ombre : alpha de la source décalé d'un nombre entier de pixels, teinté.
fragment float4 filter_shade(FilterVarying in [[stage_in]],
                             constant FilterParams& f [[buffer(0)]],
                             texture2d<float> source [[texture(0)]]) {
    int2 p = int2(in.position.xy) - int2(f.shift.xy);
    float a = fetch(source, p, f.size.xy).a * f.tint.a;
    return float4(f.tint.rgb * a, a);
}
// Contenu (texture 0) posé sur son ombre floutée (texture 1).
fragment float4 filter_merge(FilterVarying in [[stage_in]],
                             constant FilterParams& f [[buffer(0)]],
                             texture2d<float> source [[texture(0)]],
                             texture2d<float> below [[texture(1)]]) {
    int2 p = int2(in.position.xy);
    float4 top = fetch(source, p, f.size.xy);
    float4 shade = fetch(below, p, f.size.xy);
    return top + shade * (1.0 - top.a);
}
