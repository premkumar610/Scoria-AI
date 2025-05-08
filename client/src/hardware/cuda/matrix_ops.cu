// local_engine/cuda/matrix_ops.cu

#include <cuda_fp16.h>
#include <cuda_runtime.h>
#include <stdio.h>
#include <math.h>

#define CHECK_CUDA(func)                                                       \
{                                                                              \
    cudaError_t status = (func);                                               \
    if (status != cudaSuccess) {                                               \
        printf("CUDA failure at line %d: %s\n", __LINE__, cudaGetErrorString(status)); \
        exit(EXIT_FAILURE);                                                    \
    }                                                                          \
}

template <typename T>
__global__ void matrix_multiply_kernel(
    const T* A, const T* B, T* C, 
    int M, int N, int K, 
    T alpha, T beta
) {
    // Block size and tile dimensions
    const int TILE_SIZE = 16;
    __shared__ T As[TILE_SIZE][TILE_SIZE];
    __shared__ T Bs[TILE_SIZE][TILE_SIZE];

    int row = blockIdx.y * blockDim.y + threadIdx.y;
    int col = blockIdx.x * blockDim.x + threadIdx.x;

    T sum = 0.0;

    for (int t = 0; t < (K + TILE_SIZE - 1) / TILE_SIZE; ++t) {
        // Load tiles into shared memory
        if (row < M && t*TILE_SIZE + threadIdx.x < K) {
            As[threadIdx.y][threadIdx.x] = A[row*K + t*TILE_SIZE + threadIdx.x];
        } else {
            As[threadIdx.y][threadIdx.x] = 0.0;
        }

        if (t*TILE_SIZE + threadIdx.y < K && col < N) {
            Bs[threadIdx.y][threadIdx.x] = B[(t*TILE_SIZE + threadIdx.y)*N + col];
        } else {
            Bs[threadIdx.y][threadIdx.x] = 0.0;
        }

        __syncthreads();

        // Compute partial sum
        for (int k = 0; k < TILE_SIZE; ++k) {
            sum += As[threadIdx.y][k] * Bs[k][threadIdx.x];
        }

        __syncthreads();
    }

    if (row < M && col < N) {
        C[row*N + col] = alpha * sum + beta * C[row*N + col];
    }
}

__global__ void relu_activation_kernel(
    half* input, half* output, 
    int elements, half negative_slope
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < elements) {
        float val = __half2float(input[idx]);
        output[idx] = __float2half(val > 0 ? val : val * __half2float(negative_slope));
    }
}

__global__ void gelu_activation_kernel(
    float* input, float* output, 
    int elements
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < elements) {
        float x = input[idx];
        output[idx] = 0.5 * x * (1.0 + tanhf(0.7978845608 * (x + 0.044715 * x * x * x)));
    }
}

__global__ void matrix_transpose_kernel(
    const float* input, float* output,
    int rows, int cols
) {
    __shared__ float tile[32][32+1]; // +1 to avoid bank conflicts

    int x = blockIdx.x * 32 + threadIdx.x;
    int y = blockIdx.y * 32 + threadIdx.y;

    if (x < cols && y < rows) {
        tile[threadIdx.y][threadIdx.x] = input[y * cols + x];
    }

    __syncthreads();

    x = blockIdx.y * 32 + threadIdx.x;
    y = blockIdx.x * 32 + threadIdx.y;

    if (x < rows && y < cols) {
        output[y * rows + x] = tile[threadIdx.x][threadIdx.y];
    }
}

// Batch matrix multiplication (3D tensor support)
template <typename T>
__global__ void batch_matmul_kernel(
    const T* A, const T* B, T* C,
    int batch_size, int M, int N, int K,
    T alpha, T beta
) {
    extern __shared__ __align__(sizeof(T)) unsigned char shared_mem[];
    T* As = reinterpret_cast<T*>(shared_mem);
    T* Bs = As + blockDim.y * blockDim.z;

    int batch = blockIdx.z;
    int row = blockIdx.y * blockDim.y + threadIdx.y;
    int col = blockIdx.x * blockDim.x + threadIdx.x;

    T sum = 0.0;

    for (int t = 0; t < (K + blockDim.z - 1) / blockDim.z; ++t) {
        // Load tiles from global to shared memory
        if (row < M && t*blockDim.z + threadIdx.z < K) {
            As[threadIdx.y * blockDim.z + threadIdx.z] = 
                A[batch*M*K + row*K + t*blockDim.z + threadIdx.z];
        } else {
            As[threadIdx.y * blockDim.z + threadIdx.z] = 0.0;
        }

        if (t*blockDim.z + threadIdx.y < K && col < N) {
            Bs[threadIdx.z * blockDim.y + threadIdx.y] = 
                B[batch*K*N + (t*blockDim.z + threadIdx.y)*N + col];
        } else {
            Bs[threadIdx.z * blockDim.y + threadIdx.y] = 0.0;
        }

        __syncthreads();

        // Compute partial sum
        for (int k = 0; k < blockDim.z; ++k) {
            sum += As[threadIdx.y * blockDim.z + k] 
                 * Bs[k * blockDim.y + threadIdx.x];
        }

        __syncthreads();
    }

    if (row < M && col < N) {
        C[batch*M*N + row*N + col] = alpha * sum + beta * C[batch*M*N + row*N + col];
    }
}

// Rust FFI interface
extern "C" {

void cuda_matrix_multiply_f32(
    const float* A, const float* B, float* C,
    int M, int N, int K,
    float alpha, float beta,
    cudaStream_t stream
) {
    dim3 block(16, 16);
    dim3 grid((N + 15)/16, (M + 15)/16);
    matrix_multiply_kernel<<<grid, block, 0, stream>>>(A, B, C, M, N, K, alpha, beta);
}

void cuda_matrix_multiply_f16(
    const half* A, const half* B, half* C,
    int M, int N, int K,
    half alpha, half beta,
    cudaStream_t stream
) {
    dim3 block(16, 16);
    dim3 grid((N + 15)/16, (M + 15)/16);
    matrix_multiply_kernel<<<grid, block, 0, stream>>>(A, B, C, M, N, K, alpha, beta);
}

void cuda_batch_matmul_f32(
    const float* A, const float* B, float* C,
    int batch_size, int M, int N, int K,
    float alpha, float beta,
    cudaStream_t stream
) {
    dim3 block(16, 16, 4);
    dim3 grid((N + 15)/16, (M + 15)/16, batch_size);
    size_t shared_mem_size = 2 * 16*16*4 * sizeof(float);
    batch_matmul_kernel<<<grid, block, shared_mem_size, stream>>>(
        A, B, C, batch_size, M, N, K, alpha, beta
    );
}

void cuda_relu_activation_f16(
    half* input, half* output,
    int elements, half negative_slope,
    cudaStream_t stream
) {
    int block_size = 256;
    int grid_size = (elements + block_size - 1) / block_size;
    relu_activation_kernel<<<grid_size, block_size, 0, stream>>>(
        input, output, elements, negative_slope
    );
}

void cuda_gelu_activation_f32(
    float* input, float* output,
    int elements,
    cudaStream_t stream
) {
    int block_size = 256;
    int grid_size = (elements + block_size - 1) / block_size;
    gelu_activation_kernel<<<grid_size, block_size, 0, stream>>>(
        input, output, elements
    );
}

void cuda_matrix_transpose_f32(
    const float* input, float* output,
    int rows, int cols,
    cudaStream_t stream
) {
    dim3 block(32, 32);
    dim3 grid((cols + 31)/32, (rows + 31)/32);
    matrix_transpose_kernel<<<grid, block, 0, stream>>>(input, output, rows, cols);
}

} // extern "C"
