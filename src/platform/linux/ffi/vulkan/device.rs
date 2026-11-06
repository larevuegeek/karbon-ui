//! Structures Vulkan : Instance, GPU, mémoire, file et appareil logique ; géométrie (étendues, rectangles).
//! Transcrites de `vulkan_core.h` et `vulkan_wayland.h` (Vulkan 1.3) ; les assertions
//! de taille et d'alignement viennent du compilateur C sur les mêmes en-têtes.
#![allow(non_snake_case, dead_code)]
use super::*;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkAllocationCallbacks {
    pub pUserData: *mut c_void,
    pub pfnAllocation: *const c_void,
    pub pfnReallocation: *const c_void,
    pub pfnFree: *const c_void,
    pub pfnInternalAllocation: *const c_void,
    pub pfnInternalFree: *const c_void,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkApplicationInfo {
    pub sType: i32,
    pub pNext: *const c_void,
    pub pApplicationName: *const c_char,
    pub applicationVersion: u32,
    pub pEngineName: *const c_char,
    pub engineVersion: u32,
    pub apiVersion: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkInstanceCreateInfo {
    pub sType: i32,
    pub pNext: *const c_void,
    pub flags: u32,
    pub pApplicationInfo: *const VkApplicationInfo,
    pub enabledLayerCount: u32,
    pub ppEnabledLayerNames: *const *const c_char,
    pub enabledExtensionCount: u32,
    pub ppEnabledExtensionNames: *const *const c_char,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkExtensionProperties {
    pub extensionName: [c_char; 256],
    pub specVersion: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkLayerProperties {
    pub layerName: [c_char; 256],
    pub specVersion: u32,
    pub implementationVersion: u32,
    pub description: [c_char; 256],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkPhysicalDeviceLimits {
    pub maxImageDimension1D: u32,
    pub maxImageDimension2D: u32,
    pub maxImageDimension3D: u32,
    pub maxImageDimensionCube: u32,
    pub maxImageArrayLayers: u32,
    pub maxTexelBufferElements: u32,
    pub maxUniformBufferRange: u32,
    pub maxStorageBufferRange: u32,
    pub maxPushConstantsSize: u32,
    pub maxMemoryAllocationCount: u32,
    pub maxSamplerAllocationCount: u32,
    pub bufferImageGranularity: u64,
    pub sparseAddressSpaceSize: u64,
    pub maxBoundDescriptorSets: u32,
    pub maxPerStageDescriptorSamplers: u32,
    pub maxPerStageDescriptorUniformBuffers: u32,
    pub maxPerStageDescriptorStorageBuffers: u32,
    pub maxPerStageDescriptorSampledImages: u32,
    pub maxPerStageDescriptorStorageImages: u32,
    pub maxPerStageDescriptorInputAttachments: u32,
    pub maxPerStageResources: u32,
    pub maxDescriptorSetSamplers: u32,
    pub maxDescriptorSetUniformBuffers: u32,
    pub maxDescriptorSetUniformBuffersDynamic: u32,
    pub maxDescriptorSetStorageBuffers: u32,
    pub maxDescriptorSetStorageBuffersDynamic: u32,
    pub maxDescriptorSetSampledImages: u32,
    pub maxDescriptorSetStorageImages: u32,
    pub maxDescriptorSetInputAttachments: u32,
    pub maxVertexInputAttributes: u32,
    pub maxVertexInputBindings: u32,
    pub maxVertexInputAttributeOffset: u32,
    pub maxVertexInputBindingStride: u32,
    pub maxVertexOutputComponents: u32,
    pub maxTessellationGenerationLevel: u32,
    pub maxTessellationPatchSize: u32,
    pub maxTessellationControlPerVertexInputComponents: u32,
    pub maxTessellationControlPerVertexOutputComponents: u32,
    pub maxTessellationControlPerPatchOutputComponents: u32,
    pub maxTessellationControlTotalOutputComponents: u32,
    pub maxTessellationEvaluationInputComponents: u32,
    pub maxTessellationEvaluationOutputComponents: u32,
    pub maxGeometryShaderInvocations: u32,
    pub maxGeometryInputComponents: u32,
    pub maxGeometryOutputComponents: u32,
    pub maxGeometryOutputVertices: u32,
    pub maxGeometryTotalOutputComponents: u32,
    pub maxFragmentInputComponents: u32,
    pub maxFragmentOutputAttachments: u32,
    pub maxFragmentDualSrcAttachments: u32,
    pub maxFragmentCombinedOutputResources: u32,
    pub maxComputeSharedMemorySize: u32,
    pub maxComputeWorkGroupCount: [u32; 3],
    pub maxComputeWorkGroupInvocations: u32,
    pub maxComputeWorkGroupSize: [u32; 3],
    pub subPixelPrecisionBits: u32,
    pub subTexelPrecisionBits: u32,
    pub mipmapPrecisionBits: u32,
    pub maxDrawIndexedIndexValue: u32,
    pub maxDrawIndirectCount: u32,
    pub maxSamplerLodBias: f32,
    pub maxSamplerAnisotropy: f32,
    pub maxViewports: u32,
    pub maxViewportDimensions: [u32; 2],
    pub viewportBoundsRange: [f32; 2],
    pub viewportSubPixelBits: u32,
    pub minMemoryMapAlignment: usize,
    pub minTexelBufferOffsetAlignment: u64,
    pub minUniformBufferOffsetAlignment: u64,
    pub minStorageBufferOffsetAlignment: u64,
    pub minTexelOffset: i32,
    pub maxTexelOffset: u32,
    pub minTexelGatherOffset: i32,
    pub maxTexelGatherOffset: u32,
    pub minInterpolationOffset: f32,
    pub maxInterpolationOffset: f32,
    pub subPixelInterpolationOffsetBits: u32,
    pub maxFramebufferWidth: u32,
    pub maxFramebufferHeight: u32,
    pub maxFramebufferLayers: u32,
    pub framebufferColorSampleCounts: u32,
    pub framebufferDepthSampleCounts: u32,
    pub framebufferStencilSampleCounts: u32,
    pub framebufferNoAttachmentsSampleCounts: u32,
    pub maxColorAttachments: u32,
    pub sampledImageColorSampleCounts: u32,
    pub sampledImageIntegerSampleCounts: u32,
    pub sampledImageDepthSampleCounts: u32,
    pub sampledImageStencilSampleCounts: u32,
    pub storageImageSampleCounts: u32,
    pub maxSampleMaskWords: u32,
    pub timestampComputeAndGraphics: u32,
    pub timestampPeriod: f32,
    pub maxClipDistances: u32,
    pub maxCullDistances: u32,
    pub maxCombinedClipAndCullDistances: u32,
    pub discreteQueuePriorities: u32,
    pub pointSizeRange: [f32; 2],
    pub lineWidthRange: [f32; 2],
    pub pointSizeGranularity: f32,
    pub lineWidthGranularity: f32,
    pub strictLines: u32,
    pub standardSampleLocations: u32,
    pub optimalBufferCopyOffsetAlignment: u64,
    pub optimalBufferCopyRowPitchAlignment: u64,
    pub nonCoherentAtomSize: u64,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkPhysicalDeviceSparseProperties {
    pub residencyStandard2DBlockShape: u32,
    pub residencyStandard2DMultisampleBlockShape: u32,
    pub residencyStandard3DBlockShape: u32,
    pub residencyAlignedMipSize: u32,
    pub residencyNonResidentStrict: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkPhysicalDeviceProperties {
    pub apiVersion: u32,
    pub driverVersion: u32,
    pub vendorID: u32,
    pub deviceID: u32,
    pub deviceType: i32,
    pub deviceName: [c_char; 256],
    pub pipelineCacheUUID: [u8; 16],
    pub limits: VkPhysicalDeviceLimits,
    pub sparseProperties: VkPhysicalDeviceSparseProperties,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkExtent2D {
    pub width: u32,
    pub height: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkExtent3D {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkOffset2D {
    pub x: i32,
    pub y: i32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkOffset3D {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkRect2D {
    pub offset: VkOffset2D,
    pub extent: VkExtent2D,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkQueueFamilyProperties {
    pub queueFlags: u32,
    pub queueCount: u32,
    pub timestampValidBits: u32,
    pub minImageTransferGranularity: VkExtent3D,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkMemoryType {
    pub propertyFlags: u32,
    pub heapIndex: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkMemoryHeap {
    pub size: u64,
    pub flags: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkPhysicalDeviceMemoryProperties {
    pub memoryTypeCount: u32,
    pub memoryTypes: [VkMemoryType; 32],
    pub memoryHeapCount: u32,
    pub memoryHeaps: [VkMemoryHeap; 16],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkPhysicalDeviceFeatures {
    pub robustBufferAccess: u32,
    pub fullDrawIndexUint32: u32,
    pub imageCubeArray: u32,
    pub independentBlend: u32,
    pub geometryShader: u32,
    pub tessellationShader: u32,
    pub sampleRateShading: u32,
    pub dualSrcBlend: u32,
    pub logicOp: u32,
    pub multiDrawIndirect: u32,
    pub drawIndirectFirstInstance: u32,
    pub depthClamp: u32,
    pub depthBiasClamp: u32,
    pub fillModeNonSolid: u32,
    pub depthBounds: u32,
    pub wideLines: u32,
    pub largePoints: u32,
    pub alphaToOne: u32,
    pub multiViewport: u32,
    pub samplerAnisotropy: u32,
    pub textureCompressionETC2: u32,
    pub textureCompressionASTC_LDR: u32,
    pub textureCompressionBC: u32,
    pub occlusionQueryPrecise: u32,
    pub pipelineStatisticsQuery: u32,
    pub vertexPipelineStoresAndAtomics: u32,
    pub fragmentStoresAndAtomics: u32,
    pub shaderTessellationAndGeometryPointSize: u32,
    pub shaderImageGatherExtended: u32,
    pub shaderStorageImageExtendedFormats: u32,
    pub shaderStorageImageMultisample: u32,
    pub shaderStorageImageReadWithoutFormat: u32,
    pub shaderStorageImageWriteWithoutFormat: u32,
    pub shaderUniformBufferArrayDynamicIndexing: u32,
    pub shaderSampledImageArrayDynamicIndexing: u32,
    pub shaderStorageBufferArrayDynamicIndexing: u32,
    pub shaderStorageImageArrayDynamicIndexing: u32,
    pub shaderClipDistance: u32,
    pub shaderCullDistance: u32,
    pub shaderFloat64: u32,
    pub shaderInt64: u32,
    pub shaderInt16: u32,
    pub shaderResourceResidency: u32,
    pub shaderResourceMinLod: u32,
    pub sparseBinding: u32,
    pub sparseResidencyBuffer: u32,
    pub sparseResidencyImage2D: u32,
    pub sparseResidencyImage3D: u32,
    pub sparseResidency2Samples: u32,
    pub sparseResidency4Samples: u32,
    pub sparseResidency8Samples: u32,
    pub sparseResidency16Samples: u32,
    pub sparseResidencyAliased: u32,
    pub variableMultisampleRate: u32,
    pub inheritedQueries: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkPhysicalDeviceFeatures2 {
    pub sType: i32,
    pub pNext: *mut c_void,
    pub features: VkPhysicalDeviceFeatures,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkPhysicalDeviceVulkan13Features {
    pub sType: i32,
    pub pNext: *mut c_void,
    pub robustImageAccess: u32,
    pub inlineUniformBlock: u32,
    pub descriptorBindingInlineUniformBlockUpdateAfterBind: u32,
    pub pipelineCreationCacheControl: u32,
    pub privateData: u32,
    pub shaderDemoteToHelperInvocation: u32,
    pub shaderTerminateInvocation: u32,
    pub subgroupSizeControl: u32,
    pub computeFullSubgroups: u32,
    pub synchronization2: u32,
    pub textureCompressionASTC_HDR: u32,
    pub shaderZeroInitializeWorkgroupMemory: u32,
    pub dynamicRendering: u32,
    pub shaderIntegerDotProduct: u32,
    pub maintenance4: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkDeviceQueueCreateInfo {
    pub sType: i32,
    pub pNext: *const c_void,
    pub flags: u32,
    pub queueFamilyIndex: u32,
    pub queueCount: u32,
    pub pQueuePriorities: *const f32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkDeviceCreateInfo {
    pub sType: i32,
    pub pNext: *const c_void,
    pub flags: u32,
    pub queueCreateInfoCount: u32,
    pub pQueueCreateInfos: *const VkDeviceQueueCreateInfo,
    pub enabledLayerCount: u32,
    pub ppEnabledLayerNames: *const *const c_char,
    pub enabledExtensionCount: u32,
    pub ppEnabledExtensionNames: *const *const c_char,
    pub pEnabledFeatures: *const VkPhysicalDeviceFeatures,
}
const _: () =
    assert!(size_of::<VkAllocationCallbacks>() == 48 && align_of::<VkAllocationCallbacks>() == 8);
const _: () = assert!(size_of::<VkApplicationInfo>() == 48 && align_of::<VkApplicationInfo>() == 8);
const _: () =
    assert!(size_of::<VkInstanceCreateInfo>() == 64 && align_of::<VkInstanceCreateInfo>() == 8);
const _: () =
    assert!(size_of::<VkExtensionProperties>() == 260 && align_of::<VkExtensionProperties>() == 4);
const _: () =
    assert!(size_of::<VkLayerProperties>() == 520 && align_of::<VkLayerProperties>() == 4);
const _: () = assert!(
    size_of::<VkPhysicalDeviceLimits>() == 504 && align_of::<VkPhysicalDeviceLimits>() == 8
);
const _: () = assert!(
    size_of::<VkPhysicalDeviceSparseProperties>() == 20
        && align_of::<VkPhysicalDeviceSparseProperties>() == 4
);
const _: () = assert!(
    size_of::<VkPhysicalDeviceProperties>() == 824 && align_of::<VkPhysicalDeviceProperties>() == 8
);
const _: () = assert!(size_of::<VkExtent2D>() == 8 && align_of::<VkExtent2D>() == 4);
const _: () = assert!(size_of::<VkExtent3D>() == 12 && align_of::<VkExtent3D>() == 4);
const _: () = assert!(size_of::<VkOffset2D>() == 8 && align_of::<VkOffset2D>() == 4);
const _: () = assert!(size_of::<VkOffset3D>() == 12 && align_of::<VkOffset3D>() == 4);
const _: () = assert!(size_of::<VkRect2D>() == 16 && align_of::<VkRect2D>() == 4);
const _: () = assert!(
    size_of::<VkQueueFamilyProperties>() == 24 && align_of::<VkQueueFamilyProperties>() == 4
);
const _: () = assert!(size_of::<VkMemoryType>() == 8 && align_of::<VkMemoryType>() == 4);
const _: () = assert!(size_of::<VkMemoryHeap>() == 16 && align_of::<VkMemoryHeap>() == 8);
const _: () = assert!(
    size_of::<VkPhysicalDeviceMemoryProperties>() == 520
        && align_of::<VkPhysicalDeviceMemoryProperties>() == 8
);
const _: () = assert!(
    size_of::<VkPhysicalDeviceFeatures>() == 220 && align_of::<VkPhysicalDeviceFeatures>() == 4
);
const _: () = assert!(
    size_of::<VkPhysicalDeviceFeatures2>() == 240 && align_of::<VkPhysicalDeviceFeatures2>() == 8
);
const _: () = assert!(
    size_of::<VkPhysicalDeviceVulkan13Features>() == 80
        && align_of::<VkPhysicalDeviceVulkan13Features>() == 8
);
const _: () = assert!(
    size_of::<VkDeviceQueueCreateInfo>() == 40 && align_of::<VkDeviceQueueCreateInfo>() == 8
);
const _: () =
    assert!(size_of::<VkDeviceCreateInfo>() == 72 && align_of::<VkDeviceCreateInfo>() == 8);
