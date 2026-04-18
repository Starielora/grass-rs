
// TODO perhaps do this on CPU once instead of for each task shader invocation?
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

Frustum extract_frustum_from_cull_camera() {
    mat4 projview = push_constants.cull_camera.projview;

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

bool cull_meshlet_plane(vec4 plane, vec3 bounding_sphere_center, float bounding_sphere_radius) {
    return dot(plane.xyz, bounding_sphere_center) + plane.w + bounding_sphere_radius < 0;
}

bool frustum_cull(vec3 bounding_sphere_center, float bounding_sphere_radius) {
    Frustum f = extract_frustum_from_cull_camera();

    for (int i = 0; i < 6; ++i) {
        if (cull_meshlet_plane(f.planes[i], bounding_sphere_center, bounding_sphere_radius)) {
            return true;
        }
    }
    return false;
}
