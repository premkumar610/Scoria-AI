// local_engine/opencl/kernels.cl

#pragma OPENCL EXTENSION cl_khr_fp16 : enable
#pragma OPENCL EXTENSION cl_khr_byte_addressable_store : enable
#pragma OPENCL EXTENSION cl_khr_int64_base_atomics : enable

#define TILE_SIZE 32
#define SHA256_ROUNDS 64

// Matrix Operations
__kernel void matrix_multiply(
    __global const float* a,
    __global const float* b,
    __global float* c,
    const int m,
    const int n,
    const int k,
    const float alpha,
    const float beta
) {
    const int row = get_global_id(0);
    const int col = get_global_id(1);
    
    __local float a_tile[TILE_SIZE][TILE_SIZE];
    __local float b_tile[TILE_SIZE][TILE_SIZE];
    
    float sum = 0.0f;
    
    for (int t = 0; t < (k + TILE_SIZE - 1)/TILE_SIZE; ++t) {
        const int tiled = t * TILE_SIZE;
        const int a_col = tiled + get_local_id(1);
        const int b_row = tiled + get_local_id(0);
        
        a_tile[get_local_id(0)][get_local_id(1)] = (a_col < k && row < m) ? 
            a[row * k + a_col] : 0.0f;
            
        b_tile[get_local_id(0)][get_local_id(1)] = (b_row < k && col < n) ? 
            b[b_row * n + col] : 0.0f;
            
        barrier(CLK_LOCAL_MEM_FENCE);
        
        for (int i = 0; i < TILE_SIZE; ++i) {
            sum += a_tile[get_local_id(0)][i] * b_tile[i][get_local_id(1)];
        }
        
        barrier(CLK_LOCAL_MEM_FENCE);
    }
    
    if (row < m && col < n) {
        c[row * n + col] = alpha * sum + beta * c[row * n + col];
    }
}

// Activation Functions
__kernel void gelu_activation(
    __global const float* input,
    __global float* output,
    const int size
) {
    const int idx = get_global_id(0);
    if (idx >= size) return;
    
    const float x = input[idx];
    const float cdf = 0.5f * (1.0f + tanh((0.7978845608028654f * (x + 0.044715f * x * x * x))));
    output[idx] = x * cdf;
}

// Cryptographic Primitives
__kernel void sha256_merkle_proof(
    __global const uchar* data,
    __global uint* hash_tree,
    const uint data_size,
    const uint tree_depth
) {
    const uint leaf_idx = get_global_id(0);
    __private uint hash[8] = {0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 
                             0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19};
    
    // Process 64-byte chunks
    for (uint i = 0; i < data_size; i += 64) {
        process_block(data + i, hash);
    }
    
    // Store leaf hash
    const uint tree_idx = (1 << tree_depth) - 1 + leaf_idx;
    hash_tree[tree_idx * 8 + get_local_id(1)] = hash[get_local_id(1)];
    
    // Build Merkle tree
    for (uint level = 0; level < tree_depth; ++level) {
        barrier(CLK_GLOBAL_MEM_FENCE);
        const uint node_idx = leaf_idx >> (level + 1);
        if ((leaf_idx & ((1 << level) - 1)) == 0) {
            const uint left = hash_tree[(tree_idx >> level) * 8 + get_local_id(1)];
            const uint right = hash_tree[((tree_idx >> level) + 1) * 8 + get_local_id(1)];
            hash_tree[(tree_idx >> (level + 1)) * 8 + get_local_id(1)] = 
                (level == 0) ? (left + right) : sha256_compress(left, right);
        }
    }
}

// Memory Optimized Operations
__kernel void fused_matmul_gelu(
    __global const float* a,
    __global const float* b,
    __global float* c,
    const int m,
    const int n,
    const int k,
    __local float* a_local,
    __local float* b_local
) {
    const int row = get_global_id(0);
    const int col = get_global_id(1);
    const int lid = get_local_id(0) * get_local_size(1) + get_local_id(1);
    
    float sum = 0.0f;
    const int num_tiles = (k + TILE_SIZE - 1) / TILE_SIZE;
    
    for (int t = 0; t < num_tiles; ++t) {
        const int tiled_k = t * TILE_SIZE;
        const int a_col = tiled_k + get_local_id(1);
        const int b_row = tiled_k + get_local_id(0);
        
        a_local[lid] = (a_col < k && row < m) ? a[row * k + a_col] : 0.0f;
        b_local[lid] = (b_row < k && col < n) ? b[b_row * n + col] : 0.0f;
        
        barrier(CLK_LOCAL_MEM_FENCE);
        
        for (int i = 0; i < TILE_SIZE; ++i) {
            sum += a_local[get_local_id(0) * TILE_SIZE + i] * 
                   b_local[i * TILE_SIZE + get_local_id(1)];
        }
        
        barrier(CLK_LOCAL_MEM_FENCE);
    }
    
    if (row < m && col < n) {
        const float x = sum;
        const float cdf = 0.5f * (1.0f + tanh(0.7978845608028654f * (x + 0.044715f * x * x * x)));
        c[row * n + col] = x * cdf;
    }
}
