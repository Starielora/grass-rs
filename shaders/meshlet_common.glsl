
struct MeshletSharedData {
    uint meshlet_index[64];
    uint draw_index;
    uint instance_index; // gl_WorkGroupID.z
};
