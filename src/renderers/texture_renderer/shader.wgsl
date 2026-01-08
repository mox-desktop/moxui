const pi = radians(180.0);

struct Params {
    screen_resolution: vec2<u32>,
    _pad: vec2<u32>,
};
@group(1) @binding(0)
var<uniform> params: Params;

struct VertexInput {
    @location(0) position: vec2<f32>,
};

struct InstanceInput {
    @location(1) filters1: vec4<f32>,  // [opacity, brightness, contrast, saturation]
    @location(2) filters2: vec4<f32>,  // [hue_rotate, sepia, invert, grayscale]
    @location(3) rotation_depth: vec2<f32>,  // [rotation, depth]
    @location(4) scale: vec2<f32>,
    @location(5) skew: vec2<f32>,
    @location(6) rect: vec4<f32>,
    @location(7) radius: vec4<f32>,
    @location(8) texture_bounds: vec4<f32>,
    @location(9) shadow: vec3<f32>,
};

struct VertexOutput {
    @location(0) layer: u32,
    @location(1) opacity: f32,
    @location(2) rotation: f32,
    @location(3) brightness: f32,
    @location(4) contrast: f32,
    @location(5) saturation: f32,
    @location(6) hue_rotate: f32,
    @location(7) sepia: f32,
    @location(8) invert: f32,
    @location(9) grayscale: f32,
    @location(10) tex_coords: vec2<f32>,
    @location(11) size: vec2<f32>,
    @location(12) surface_position: vec2<f32>,
    @location(13) shadow_softness: f32,
    @location(14) shadow_offset: vec2<f32>,
    @location(15) radius: vec4<f32>,
    @location(16) texture_bounds: vec4<f32>,
    @builtin(position) clip_position: vec4<f32>,
};

fn rotation_matrix(angle: f32) -> mat2x2<f32> {
    let angle_inner = angle * pi / 180.0;
    let sinTheta = sin(angle_inner);
    let cosTheta = cos(angle_inner);
    return mat2x2<f32>(
        cosTheta, -sinTheta,
        sinTheta, cosTheta
    );
}

fn skew_matrix(skew_x: f32, skew_y: f32) -> mat2x2<f32> {
    return mat2x2<f32>(
        vec2<f32>(1.0, skew_y * pi / 180.0),
        vec2<f32>(skew_x * pi / 180.0, 1.0)
    );
}

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
    @builtin(instance_index) instance_idx: u32,
) -> VertexOutput {
    var out: VertexOutput;

    // Unpack data
    let opacity = instance.filters1.x;
    let brightness = instance.filters1.y;
    let contrast = instance.filters1.z;
    let saturation = instance.filters1.w;
    
    let hue_rotate = instance.filters2.x;
    let sepia = instance.filters2.y;
    let invert = instance.filters2.z;
    let grayscale = instance.filters2.w;
    
    let rotation = instance.rotation_depth.x;
    let depth = instance.rotation_depth.y;

    let pos = instance.rect.xy * instance.scale;
    let size = instance.rect.zw * instance.scale;

    let local_pos = (model.position - vec2<f32>(0.5)) * size;
    let rotated_pos = rotation_matrix(rotation) * local_pos;
    let position = rotated_pos + pos + size * 0.5;

    // Convert from pixel coordinates to NDC (like shape_renderer)
    let resolution = vec2<f32>(params.screen_resolution);
    let ndc = (position / resolution) * 2.0 - vec2<f32>(1.0, 1.0);
    let ndc_fixed = vec2<f32>(ndc.x, -ndc.y);

    out.clip_position = vec4<f32>(ndc_fixed, depth, 1.0);
    out.tex_coords = model.position;
    out.layer = instance_idx;
    out.size = size;
    out.texture_bounds = instance.texture_bounds;
    out.surface_position = position;
    out.opacity = opacity;
    out.rotation = rotation;
    out.radius = instance.radius;
    out.brightness = brightness;
    out.contrast = contrast;
    out.saturation = saturation;
    out.hue_rotate = hue_rotate;
    out.sepia = sepia;
    out.invert = invert;
    out.grayscale = grayscale;
    out.shadow_offset = instance.shadow.xy;
    out.shadow_softness = instance.shadow.z;

    return out;
}

fn sdf_rounded_rect(p: vec2<f32>, b: vec2<f32>, r: vec4<f32>) -> f32 {
    var x = select(r.x, r.y, p.x > 0.0);
    var y = select(r.z, r.w, p.x > 0.0);
    let radius = select(y, x, p.y > 0.0);
    let q = abs(p) - b + radius;
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0))) - radius;
}

fn brightness_matrix(brightness: f32) -> mat4x4<f32> {
    return mat4x4<f32>(
        1, 0, 0, 0,
        0, 1, 0, 0,
        0, 0, 1, 0,
        brightness, brightness, brightness, 1
    );
}

fn contrast_matrix(contrast: f32) -> mat4x4<f32> {
    let t = (1.0 - contrast) / 2.0;
    return mat4x4<f32>(
        contrast, 0, 0, 0,
        0, contrast, 0, 0,
        0, 0, contrast, 0,
        t, t, t, 1
    );
}

fn saturation_matrix(saturation: f32) -> mat4x4<f32> {
    let luminance = vec3<f32>(0.3086, 0.6094, 0.0820);
    let one_minus_sat = 1.0 - saturation;

    var red: vec3<f32> = vec3<f32>(luminance.x * one_minus_sat);
    red += vec3<f32>(saturation, 0, 0);

    var green: vec3<f32> = vec3<f32>(luminance.y * one_minus_sat);
    green += vec3<f32>(0, saturation, 0);

    var blue: vec3<f32> = vec3<f32>(luminance.z * one_minus_sat);
    blue += vec3<f32>(0, 0, saturation);

    return mat4x4<f32>(
        vec4<f32>(red, 0.0),
        vec4<f32>(green, 0.0),
        vec4<f32>(blue, 0.0),
        vec4<f32>(0.0, 0.0, 0.0, 1.0)
    );
}

fn grayscale(color: vec3<f32>, intensity: f32) -> vec3<f32> {
    let luminance = dot(color, vec3<f32>(0.299, 0.587, 0.114));
    return mix(color, vec3<f32>(luminance), intensity);
}

fn sepia(color: vec3<f32>, sepia: f32) -> vec3<f32> {
    let sepia_matrix = vec3<f32>(
        dot(color.rgb, vec3<f32>(0.393, 0.769, 0.189)),
        dot(color.rgb, vec3<f32>(0.349, 0.686, 0.168)),
        dot(color.rgb, vec3<f32>(0.272, 0.534, 0.131))
    );
    return mix(color.rgb, sepia_matrix, sepia);
}

fn hue_rotate(color: vec3<f32>, angle: f32) -> vec3<f32> {
    return vec3<f32>(
        dot(color, vec3<f32>(0.213, 0.715, -0.213)) * (1.0 - cos(angle)) + cos(angle) * color.r + sin(angle) * color.b,
        dot(color, vec3<f32>(-0.213, 0.715, 0.715)) * (1.0 - cos(angle)) + cos(angle) * color.g + sin(angle) * color.g,
        dot(color, vec3<f32>(0.272, -0.715, 0.213)) * (1.0 - cos(angle)) + cos(angle) * color.b + sin(angle) * color.r
    );
}

@group(0) @binding(0)
var t_diffuse: texture_2d_array<f32>; 
@group(0) @binding(1)
var s_diffuse: sampler;

fn gaussian_shadow(dist: f32, blur_radius: f32) -> f32 {
    if blur_radius <= 0.0 {
        return select(0.0, 1.0, dist <= 0.0);
    }
    
    let normalized_dist = abs(dist) / blur_radius;
    let gaussian = exp(-normalized_dist * normalized_dist * 0.5);
    let falloff = smoothstep(0.0, 1.0, 1.0 - normalized_dist);

    return gaussian * falloff;
}

fn is_outside_container(surface_pos: vec2<f32>, texture_bounds: vec4<f32>) -> bool {
    let container_left = texture_bounds.x;
    let container_top = texture_bounds.y;
    let container_right = texture_bounds.z;
    let container_bottom = texture_bounds.w;

    return surface_pos.x < container_left || surface_pos.x > container_right || surface_pos.y < container_top || surface_pos.y > container_bottom;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Clip to container bounds
    if is_outside_container(in.surface_position, in.texture_bounds) {
        discard;
    }

    // Sample texture (straight alpha)
    let base_color = textureSample(t_diffuse, s_diffuse, in.tex_coords, in.layer);
  
    // === ROUNDED CORNERS ===
    let centered_tex_coords = in.tex_coords - 0.5;
    let half_extent = vec2<f32>(0.5, 0.5);
    let texture_radius = in.radius * 0.01;
    let max_radius = vec4<f32>(half_extent.x, half_extent.x, half_extent.y, half_extent.y);
    let effective_radius = min(texture_radius, max_radius);
    let texture_dist = sdf_rounded_rect(centered_tex_coords, half_extent, effective_radius);
    let texture_aa = fwidth(texture_dist) * 0.6;
    let texture_alpha = smoothstep(-texture_aa, texture_aa, -texture_dist);

    // === SHADOW ===
    let shadow_offset_normalized = in.shadow_offset / in.size;
    let shadow_coords = centered_tex_coords - shadow_offset_normalized;
    let shadow_dist = sdf_rounded_rect(shadow_coords, half_extent + (in.shadow_offset / in.size) / 2., effective_radius);
    let shadow_alpha = gaussian_shadow(shadow_dist, in.shadow_softness / min(in.size.x, in.size.y));

    // === COLOR FILTERS ===
    var final_rgb = base_color.rgb;
    
    // Check if we need to apply color filters
    let needs_filters = in.brightness != 0.0 || in.contrast != 1.0 || in.saturation != 1.0 || 
                       in.hue_rotate != 0.0 || in.sepia != 0.0 || in.grayscale != 0.0 || in.invert != 0.0;
    
    if needs_filters {
        // Apply color matrix filters
        var filtered_color = vec4<f32>(final_rgb, 1.0);
        filtered_color = brightness_matrix(in.brightness) * contrast_matrix(in.contrast) * saturation_matrix(in.saturation) * filtered_color;
        
        // Apply other color transforms  
        let hue_rotated = hue_rotate(filtered_color.rgb, in.hue_rotate);
        let sepia_applied = sepia(hue_rotated, in.sepia);
        let gray_applied = grayscale(sepia_applied, in.grayscale);
        final_rgb = mix(gray_applied, vec3<f32>(1.0) - gray_applied, in.invert);
    }

    // Apply opacity and rounded corners to alpha
    let final_alpha = base_color.a * texture_alpha * in.opacity;
    
    // Premultiply and return (shadow disabled for now to maintain transparency)
    return vec4<f32>(final_rgb * final_alpha, final_alpha);
}
