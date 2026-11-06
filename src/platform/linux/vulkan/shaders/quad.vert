#version 450
// Un quad par instance (bande de deux triangles, quatre sommets sans tampon).
#extension GL_GOOGLE_include_directive : require
#include "common.glsl"

layout(location = 0) out vec2 local;
layout(location = 1) out vec2 uv;
layout(location = 2) flat out uint instance;

void main() {
    Instance i = instances[gl_InstanceIndex];
    vec2 unit = vec2(float(gl_VertexIndex & 1), float(gl_VertexIndex >> 1));
    vec2 l = mix(i.quad.xy, i.quad.zw, unit);
    vec2 pixel = vec2(l.x * i.m.x + l.y * i.m.z + i.t.x, l.x * i.m.y + l.y * i.m.w + i.t.y);
    // Vulkan : l'axe y du NDC descend, comme les pixels (pas d'inversion, contrairement
    // à Metal).
    vec2 ndc = (pixel - frame.origin) / frame.size * 2.0 - 1.0;
    gl_Position = vec4(ndc, 0.0, 1.0);
    local = l;
    uv = mix(i.uv.xy, i.uv.zw, unit);
    instance = uint(gl_InstanceIndex);
}
