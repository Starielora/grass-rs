struct Frustum {
    vec4 planes[6];
};

vec4 get_row(mat4 m, int i) {
    return vec4(m[0][i], m[1][i], m[2][i], m[3][i]);
}

vec4 normalize_plane(vec4 p) {
    vec4 normalized = p / length(p.xyz);
    return normalized;
}

Frustum extract_frustum(mat4 projview) {
    vec4 r1 = get_row(projview, 0);
    vec4 r2 = get_row(projview, 1);
    vec4 r3 = get_row(projview, 2);
    vec4 r4 = get_row(projview, 3);

    Frustum f;
    f.planes[0] = normalize_plane(r4 + r1); // left
    f.planes[1] = normalize_plane(r4 - r1); // right
    f.planes[2] = normalize_plane(r4 + r2); // bottom
    f.planes[3] = normalize_plane(r4 - r2); // top
    f.planes[4] = normalize_plane(r3); // near <- [0,1] depth, no r4+r3
    f.planes[5] = normalize_plane(r4 - r3); // far

    return f;
}

bool cull_plane(vec4 plane, vec3 bounding_sphere_center, float bounding_sphere_radius) {
    return dot(plane.xyz, bounding_sphere_center) + plane.w + bounding_sphere_radius < 0;
}

bool frustum_cull(mat4 projview, vec3 bounding_sphere_center, float bounding_sphere_radius) {
    Frustum f = extract_frustum(projview);

    for (int i = 0; i < 6; ++i) {
        if (cull_plane(f.planes[i], bounding_sphere_center, bounding_sphere_radius)) {
            return true;
        }
    }
    return false;
}

vec4 compute_pos_world(mat4 transform, vec3 center) {
    return transform * vec4(center, 1.0);
}

float compute_radius_world(mat4 transform, float radius) {
    return radius * length(transform[0].xyz);
    
}
