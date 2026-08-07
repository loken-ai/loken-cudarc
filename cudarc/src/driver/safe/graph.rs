use std::sync::Arc;

use crate::driver::{result, sys};

use super::{CudaStream, DriverError};

/// Represents a replay-able Cuda Graph. Create with [CudaStream::begin_capture()] and [CudaStream::end_capture()].
///
/// Once created you can replay with [CudaGraph::launch()].
///
/// # On Thread safety
///
/// This object is **NOT** thread safe.
///
/// From official docs:
///
/// > Graph objects (cudaGraph_t, CUgraph) are not internally synchronized and must not be accessed concurrently from multiple threads. API calls accessing the same graph object must be serialized externally.
/// >
/// > Note that this includes APIs which may appear to be read-only, such as cudaGraphClone() (cuGraphClone()) and cudaGraphInstantiate() (cuGraphInstantiate()). No API or pair of APIs is guaranteed to be safe to call on the same graph object from two different threads without serialization.
///
/// <https://docs.nvidia.com/cuda/cuda-driver-api/graphs-thread-safety.html#graphs-thread-safety>
pub struct CudaGraph {
    cu_graph: sys::CUgraph,
    cu_graph_exec: sys::CUgraphExec,
    stream: Arc<CudaStream>,
}

impl Drop for CudaGraph {
    fn drop(&mut self) {
        let ctx = &self.stream.ctx;

        let cu_graph_exec = std::mem::replace(&mut self.cu_graph_exec, std::ptr::null_mut());
        if !cu_graph_exec.is_null() {
            ctx.record_err(unsafe { result::graph::exec_destroy(cu_graph_exec) });
        }

        let cu_graph = std::mem::replace(&mut self.cu_graph, std::ptr::null_mut());
        if !cu_graph.is_null() {
            ctx.record_err(unsafe { result::graph::destroy(cu_graph) });
        }
    }
}

impl CudaStream {
    /// See [cuda docs](https://docs.nvidia.com/cuda/cuda-driver-api/group__CUDA__STREAM.html#group__CUDA__STREAM_1g767167da0bbf07157dc20b6c258a2143)
    pub fn begin_capture(&self, mode: sys::CUstreamCaptureMode) -> Result<(), DriverError> {
        self.ctx.bind_to_thread()?;
        unsafe { result::stream::begin_capture(self.cu_stream, mode) }
    }

    /// See [cuda docs](https://docs.nvidia.com/cuda/cuda-driver-api/group__CUDA__STREAM.html#group__CUDA__STREAM_1g03dab8b2ba76b00718955177a929970c)
    ///
    /// `flags` is passed to [cuGraphInstantiate](https://docs.nvidia.com/cuda/cuda-driver-api/group__CUDA__GRAPH.html#group__CUDA__GRAPH_1gb53b435e178cccfa37ac87285d2c3fa1)
    pub fn end_capture(
        self: &Arc<Self>,
        flags: sys::CUgraphInstantiate_flags,
    ) -> Result<Option<CudaGraph>, DriverError> {
        self.ctx.bind_to_thread()?;
        let cu_graph = unsafe { result::stream::end_capture(self.cu_stream) }?;
        if cu_graph.is_null() {
            return Ok(None);
        }
        let cu_graph_exec = unsafe { result::graph::instantiate(cu_graph, flags) }?;
        Ok(Some(CudaGraph {
            cu_graph,
            cu_graph_exec,
            stream: self.clone(),
        }))
    }

    /// See [cuda docs](https://docs.nvidia.com/cuda/cuda-driver-api/group__CUDA__STREAM.html#group__CUDA__STREAM_1g37823c49206e3704ae23c7ad78560bca)
    pub fn capture_status(&self) -> Result<sys::CUstreamCaptureStatus, DriverError> {
        self.ctx.bind_to_thread()?;
        unsafe { result::stream::is_capturing(self.cu_stream) }
    }
}

impl CudaGraph {
    /// See [cuda docs](https://docs.nvidia.com/cuda/cuda-driver-api/group__CUDA__GRAPH.html#group__CUDA__GRAPH_1g6b2dceb3901e71a390d2bd8b0491e471)
    pub fn launch(&self) -> Result<(), DriverError> {
        self.stream.ctx.bind_to_thread()?;
        unsafe { result::graph::launch(self.cu_graph_exec, self.stream.cu_stream) }
    }

    /// Pre-uploads the graph's resources to the device so that the
    /// first [CudaGraph::launch()] does not incur setup overhead.
    ///
    /// See [cuda docs](https://docs.nvidia.com/cuda/cuda-driver-api/group__CUDA__GRAPH.html#group__CUDA__GRAPH_1gdb81438b083d42a26693f6f2bce150cd)
    pub fn upload(&self) -> Result<(), DriverError> {
        self.stream.ctx.bind_to_thread()?;
        unsafe { result::graph::upload(self.cu_graph_exec, self.stream.cu_stream) }
    }

    /// Get the underlying [sys::CUgraph].
    ///
    /// # Safety
    /// While this function is marked as safe, actually using the
    /// returned object is unsafe.
    ///
    /// **You must not destroy the graph**, as it is still
    /// owned by the [CudaGraph].
    pub fn cu_graph(&self) -> sys::CUgraph {
        self.cu_graph
    }

    /// Returns the number of nodes in the captured graph.
    /// See [cuda docs](https://docs.nvidia.com/cuda/cuda-driver-api/group__CUDA__GRAPH.html#group__CUDA__GRAPH_1g7cd6c92fbd3dffc9b0d72bca5e7f8e02)
    pub fn num_nodes(&self) -> Result<usize, DriverError> {
        self.stream.ctx.bind_to_thread()?;
        let mut count: usize = 0;
        unsafe {
            sys::cuGraphGetNodes(
                self.cu_graph,
                std::ptr::null_mut(),
                &mut count,
            ).result()?;
        }
        Ok(count)
    }

    /// Get the underlying [sys::CUgraphExec].
    ///
    /// # Safety
    /// While this function is marked as safe, actually using the
    /// returned object is unsafe.
    ///
    /// **You must not destroy the graph exec**, as it is still
    /// owned by the [CudaGraph].
    pub fn cu_graph_exec(&self) -> sys::CUgraphExec {
        self.cu_graph_exec
    }

    /// Enumerate the graph's nodes. Returns up to `cap` node handles.
    /// Pass `cap = self.num_nodes()?` to get all of them.
    /// See [cuda docs](https://docs.nvidia.com/cuda/cuda-driver-api/group__CUDA__GRAPH.html#group__CUDA__GRAPH_1g7cd6c92fbd3dffc9b0d72bca5e7f8e02)
    pub fn nodes(&self, cap: usize) -> Result<Vec<sys::CUgraphNode>, DriverError> {
        self.stream.ctx.bind_to_thread()?;
        let mut nodes: Vec<sys::CUgraphNode> = vec![std::ptr::null_mut(); cap];
        let mut count = cap;
        unsafe {
            sys::cuGraphGetNodes(
                self.cu_graph,
                nodes.as_mut_ptr(),
                &mut count,
            ).result()?;
        }
        nodes.truncate(count);
        Ok(nodes)
    }

    /// Returns the node type for each node in the captured graph,
    /// in `nodes()` order. Filters out non-kernel nodes (memcpy,
    /// memset, etc.) so the caller can build a kernel-node-only
    /// index for patching.
    pub fn node_types(&self, nodes: &[sys::CUgraphNode]) -> Result<Vec<sys::CUgraphNodeType>, DriverError> {
        self.stream.ctx.bind_to_thread()?;
        let mut out = Vec::with_capacity(nodes.len());
        for &n in nodes {
            let mut ty = sys::CUgraphNodeType::CU_GRAPH_NODE_TYPE_KERNEL;
            unsafe {
                sys::cuGraphNodeGetType(n, &mut ty).result()?;
            }
            out.push(ty);
        }
        Ok(out)
    }

    /// Read a kernel node's current params (function handle + grid/block
    /// + arg buffer). Used to identify which kernel a captured node
    /// runs, so the engine can match against a per-arch registry of
    /// position-dependent kernels.
    ///
    /// # Safety
    /// `params` must point to a writable `CUDA_KERNEL_NODE_PARAMS`
    /// that will be filled by the driver. The arg pointer values
    /// returned are owned by the captured graph — do not free.
    pub unsafe fn kernel_node_get_params(
        &self,
        node: sys::CUgraphNode,
        params: *mut sys::CUDA_KERNEL_NODE_PARAMS,
    ) -> Result<(), DriverError> {
        self.stream.ctx.bind_to_thread()?;
        sys::cuGraphKernelNodeGetParams_v2(node, params).result()
    }

    /// Update a kernel node's parameters in the executable graph
    /// in-place (no re-instantiation, no re-capture). Use this to
    /// patch per-token pointers (KV cache positions, sampling
    /// destinations) between replays.
    ///
    /// Pattern: llama.cpp's per-token node patching via
    /// `cudaGraphExecKernelNodeSetParams`. Cheap relative to the
    /// alternative (re-record + cuGraphExecUpdate which still
    /// rebuilds the whole graph structure).
    ///
    /// # Safety
    /// `node` must be a kernel node from `self.nodes()` and
    /// `params` must point to a fully initialised
    /// `CUDA_KERNEL_NODE_PARAMS` describing the same kernel
    /// (same func handle + grid/block) — only the arg pointer
    /// values may change.
    pub unsafe fn exec_kernel_node_set_params(
        &self,
        node: sys::CUgraphNode,
        params: *const sys::CUDA_KERNEL_NODE_PARAMS,
    ) -> Result<(), DriverError> {
        self.stream.ctx.bind_to_thread()?;
        sys::cuGraphExecKernelNodeSetParams_v2(self.cu_graph_exec, node, params).result()
    }

    /// Re-capture the stream into a fresh `CUgraph` and patch this
    /// instance's executable in place via `cuGraphExecUpdate`. The
    /// caller must have just performed `begin_capture` + work + this
    /// function will end-capture and either update or fall back to a
    /// fresh instantiate if the structural diff prevents update.
    ///
    /// Returns `Ok(true)` if `cuGraphExecUpdate` succeeded (cheap
    /// path), `Ok(false)` if the graph had to be re-instantiated
    /// (expensive path). The capture must have been initiated on
    /// `self.stream` with a fresh `begin_capture` before this call.
    ///
    /// Pattern: llama.cpp ggml-cuda.cu's `ggml_cuda_graph_update_executable`.
    /// Lets per-token captures (with new pointers) replace the baked
    /// pointers in the instantiated exec graph without paying full
    /// instantiate cost on every token.
    pub fn end_capture_and_update(&mut self) -> Result<bool, DriverError> {
        self.stream.ctx.bind_to_thread()?;
        let new_graph = unsafe { result::stream::end_capture(self.stream.cu_stream) }?;
        if new_graph.is_null() {
            return Err(DriverError(
                sys::CUresult::CUDA_ERROR_INVALID_VALUE,
            ));
        }
        // Try in-place update first. CUDA 12+ uses the _v2 API with
        // a single result-info struct; older CUDA 11.x uses out-params.
        let mut result_info = sys::CUgraphExecUpdateResultInfo_st {
            result: sys::CUgraphExecUpdateResult::CU_GRAPH_EXEC_UPDATE_SUCCESS,
            errorNode: std::ptr::null_mut(),
            errorFromNode: std::ptr::null_mut(),
        };
        let stat = unsafe {
            sys::cuGraphExecUpdate_v2(
                self.cu_graph_exec,
                new_graph,
                &mut result_info,
            )
        };
        if stat == sys::CUresult::CUDA_SUCCESS
            && result_info.result == sys::CUgraphExecUpdateResult::CU_GRAPH_EXEC_UPDATE_SUCCESS
        {
            // Update succeeded — drop the old captured graph (the
            // exec is now backed by the new structure).
            unsafe { result::graph::destroy(self.cu_graph) }?;
            self.cu_graph = new_graph;
            Ok(true)
        } else {
            // Update failed (graph structure changed in an
            // incompatible way). Drop the freshly captured graph
            // and surface the error — the caller should fall back
            // to a full `end_capture` cycle which discards and
            // re-instantiates.
            let _ = unsafe { result::graph::destroy(new_graph) };
            Err(DriverError(stat))
        }
    }

    /// Like `end_capture_and_update` but on UPDATE failure, transparently
    /// destroys the stale exec graph and re-instantiates a fresh one from
    /// the newly captured graph. Returns `UpdateOutcome::Updated` (cheap
    /// pointer patch — the previous instantiation lives on), or
    /// `UpdateOutcome::Reinstantiated` (full exec rebuild — slower but
    /// handles structural diffs that the in-place update can't).
    ///
    /// Used by archs whose per-token forward graph changes structurally
    /// (phi2: matmul kernel function-handle drift across captures because
    /// cuBLAS auto-selects per call). The re-instantiate cost is
    /// typically 50-200 µs for a 2-3K-node graph — still a win versus
    /// paying per-launch overhead for 12+ launches per layer × 24 layers
    /// (~3 ms/token saved). For arches where update succeeds, this is
    /// identical to `end_capture_and_update`.
    pub fn end_capture_or_reinstantiate(
        &mut self,
    ) -> Result<UpdateOutcome, DriverError> {
        self.stream.ctx.bind_to_thread()?;
        let new_graph = unsafe { result::stream::end_capture(self.stream.cu_stream) }?;
        if new_graph.is_null() {
            return Err(DriverError(
                sys::CUresult::CUDA_ERROR_INVALID_VALUE,
            ));
        }
        let mut result_info = sys::CUgraphExecUpdateResultInfo_st {
            result: sys::CUgraphExecUpdateResult::CU_GRAPH_EXEC_UPDATE_SUCCESS,
            errorNode: std::ptr::null_mut(),
            errorFromNode: std::ptr::null_mut(),
        };
        let stat = unsafe {
            sys::cuGraphExecUpdate_v2(
                self.cu_graph_exec,
                new_graph,
                &mut result_info,
            )
        };
        if stat == sys::CUresult::CUDA_SUCCESS
            && result_info.result == sys::CUgraphExecUpdateResult::CU_GRAPH_EXEC_UPDATE_SUCCESS
        {
            unsafe { result::graph::destroy(self.cu_graph) }?;
            self.cu_graph = new_graph;
            Ok(UpdateOutcome::Updated)
        } else {
            // Structural diff: destroy stale exec, instantiate fresh
            // from the new captured graph. Same flags as initial
            // instantiate (zero — default).
            let flags: sys::CUgraphInstantiate_flags = unsafe {
                std::mem::transmute::<u32, _>(0u32)
            };
            let new_exec = match unsafe { result::graph::instantiate(new_graph, flags) } {
                Ok(e) => e,
                Err(e) => {
                    let _ = unsafe { result::graph::destroy(new_graph) };
                    return Err(e);
                }
            };
            // Destroy the OLD exec before swapping in the new one.
            let _ = unsafe { result::graph::exec_destroy(self.cu_graph_exec) };
            self.cu_graph_exec = new_exec;
            // Destroy the OLD captured graph; the new captured graph
            // is what backs the new exec — keep it.
            let _ = unsafe { result::graph::destroy(self.cu_graph) };
            self.cu_graph = new_graph;
            Ok(UpdateOutcome::Reinstantiated)
        }
    }
}

/// Outcome of `end_capture_or_reinstantiate`: either the cheap
/// in-place update succeeded, or the structural diff forced a full
/// re-instantiate. Both are valid; the caller can use the value for
/// telemetry / heuristic gating.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateOutcome {
    Updated,
    Reinstantiated,
}
