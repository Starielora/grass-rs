struct TaskPayload {
    // This is insane, glsl. I wanted a specialized constant for this size and reuse it for local_size_x,
    // but apparently glsl sets local_size_x to a default 1, when used for local_size_x_id - does not respect the default value of actual constant, but it uses 64 for array size, wtf.
    // Instead, I'm going for a worst case scenario - 64 or 128 elements, which will fit all cases, and use gl_WorkGroupSize.x to query chosen wokrgroup size
    // ... because constant would not solve my issues, as apparently AMD GPU driver can dynamically choose a subgroup size of EITHER 32 or 64
    // And thinking about managing the requiredSubgroupSize VK extension with all that just made me give up and go as simple as possible.
    uint meshlet_instance_index[64]; // must be >= task shader local_size_x
};
