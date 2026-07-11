struct Sphere {
    vec3 center;
    float radius;
};

struct TaskPayload {
    Sphere spheres[64];
};
