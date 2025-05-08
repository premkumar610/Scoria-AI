// local_engine/src/cuda/ffi.rs

use std::{
    ffi::c_void,
    os::raw::c_int,
    sync::{Arc, Mutex},
    ptr,
};
use nvml_wrapper::Nvml;
use thiserror::Error;
use half::f16;
use cust::{
    memory::DeviceBox,
    stream::{Stream, StreamFlags},
};

#[derive(Debug, Error)]
pub enum CudaError {
    #[error("CUDA driver error: {0}")]
    DriverError(#[from] cust::error::CudaError),
    #[error("Memory allocation failed for {operation}")]
    AllocationError { operation: String },
    #[error("Dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: String, actual: String },
    #[error("Batch size mismatch: expected {expected}, got {actual}")]
    BatchMismatch { expected: usize, actual: usize },
    #[error("Unsupported precision: {0}")]
    UnsupportedPrecision(String),
}

#[repr(C)]
pub struct CudaContext {
    stream: Stream,
    device_memory: Arc<Mutex<u64>>,
}

impl CudaContext {
    pub fn new() -> Result<Self, CudaError> {
        let flags = StreamFlags::NON_BLOCKING;
        let stream = Stream::new(flags)?;
        
        // Initialize NVML for GPU monitoring
        let nvml = Nvml::init()?;
        let device = nvml.device_by_index(0)?;
        let mem_info = device.memory_info()?;

        Ok(Self {
            stream,
            device_memory: Arc::new(Mutex::new(mem_info.free)),
        })
    }

    pub fn create_stream(&self) -> Result<*mut c_void, CudaError> {
        let flags = StreamFlags::NON_BLOCKING;
        let stream = Stream::new(flags)?;
        Ok(stream.as_ptr() as *mut c_void)
    }
}

#[repr(C)]
pub struct DeviceTensor<T> {
    data: DeviceBox<T>,
    shape: Vec<usize>,
    strides: Vec<usize>,
    context: Arc<CudaContext>,
}

impl<T> DeviceTensor<T> {
    pub fn new(
        host_data: &[T],
        shape: &[usize],
        context: Arc<CudaContext>,
    ) -> Result<Self, CudaError>
    where
        T: Copy,
    {
        let mut data = DeviceBox::new(host_data).map_err(|e| CudaError::AllocationError {
            operation: "DeviceBox::new".to_string(),
        })?;

        let strides = compute_strides(shape);
        
        // Update memory tracker
        let bytes_needed = host_data.len() * std::mem::size_of::<T>();
        let mut mem_guard = context.device_memory.lock().unwrap();
        if *mem_guard < bytes_needed as u64 {
            return Err(CudaError::AllocationError {
                operation: format!("Require {} bytes", bytes_needed),
            });
        }
        *mem_guard -= bytes_needed as u64;

        Ok(Self {
            data,
            shape: shape.to_vec(),
            strides,
            context,
        })
    }
}

impl<T> Drop for DeviceTensor<T> {
    fn drop(&mut self) {
        // Release memory tracking
        let bytes_freed = self.data.len() * std::mem::size_of::<T>();
        let mut mem_guard = self.context.device_memory.lock().unwrap();
        *mem_guard += bytes_freed as u64;
    }
}

#[no_mangle]
pub extern "C" fn cuda_matrix_multiply_f32(
    a: *const f32,
    b: *const f32,
    c: *mut f32,
    m: c_int,
    n: c_int,
    k: c_int,
    alpha: f32,
    beta: f32,
    stream: *mut c_void,
) -> i32 {
    let stream = unsafe { Stream::from_ptr(stream as *mut cust::stream::CudaStream) };
    
    let result = unsafe {
        let a_slice = std::slice::from_raw_parts(a, (m * k) as usize);
        let b_slice = std::slice::from_raw_parts(b, (k * n) as usize);
        let mut c_slice = std::slice::from_raw_parts_mut(c, (m * n) as usize);

        let a_dev = match DeviceBox::new(a_slice) {
            Ok(d) => d,
            Err(_) => return CudaError::AllocationError { operation: "A matrix".to_string() } as i32,
        };
        
        let b_dev = match DeviceBox::new(b_slice) {
            Ok(d) => d,
            Err(_) => return CudaError::AllocationError { operation: "B matrix".to_string() } as i32,
        };
        
        let mut c_dev = match DeviceBox::new(&c_slice) {
            Ok(d) => d,
            Err(_) => return CudaError::AllocationError { operation: "C matrix".to_string() } as i32,
        };

        let grid = cust::launch_cfg!((n as u32 / 16 + 1, m as u32 / 16 + 1), 256);
        
        match launch!(matrix_multiply_kernel<<<grid, stream>>>(
            a_dev.as_device_ptr(),
            b_dev.as_device_ptr(),
            c_dev.as_device_ptr(),
            m,
            n,
            k,
            alpha,
            beta
        )) {
            Ok(_) => {
                c_dev.copy_to(&mut c_slice).unwrap();
                0
            }
            Err(e) => e.raw as i32,
        }
    };

    result
}

// Similar implementations for:
// - cuda_matrix_multiply_f16
// - cuda_batch_matmul_f32 
// - cuda_relu_activation_f16
// - cuda_gelu_activation_f32
// - cuda_matrix_transpose_f32

#[no_mangle]
pub extern "C" fn create_cuda_context() -> *mut CudaContext {
    match CudaContext::new() {
        Ok(ctx) => Box::into_raw(Box::new(ctx)),
        Err(_) => ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn destroy_cuda_context(ctx: *mut CudaContext) {
    if !ctx.is_null() {
        unsafe { Box::from_raw(ctx) };
    }
}

fn compute_strides(shape: &[usize]) -> Vec<usize> {
    let ndim = shape.len();
    let mut strides = vec![1; ndim];
    
    for i in (0..ndim - 1).rev() {
        strides[i] = strides[i + 1] * shape[i + 1];
    }
    
    strides
}
