uniform sampler2D al_tex;
varying vec2 varying_texcoord;

uniform sampler2D distance_map;
uniform int num_rays;
uniform int num_steps;

// RC only.
uniform sampler2D prev_cascade;
uniform float base;
uniform vec2 bitmap_size;
uniform float cascade_index;
uniform float num_cascades;
uniform int last_index;

const float PI = 3.141592653;
const float EPS = 0.001;
const float POWER = 1.6;

vec2 flip_y(vec2 uv)
{
    return vec2(uv.x, 1. - uv.y);
}

vec4 basic_raycasting()
{
    vec2 scaled_uv = flip_y(varying_texcoord);
    vec4 light = texture2D(al_tex, flip_y(scaled_uv));

    vec4 radiance = vec4(0.);
    float delta_theta = 2. * PI / float(num_rays);

    if (light.a < 0.1)
    {
        for (int i = 0; i < num_rays; i++)
        {
            float theta = delta_theta * float(i);
            vec2 dir = vec2(cos(theta), sin(theta));
            
            vec2 src_uv = scaled_uv;
            for (int step = 1; step < num_steps; step++)
            {
                float dist = texture2D(distance_map, flip_y(src_uv)).r;
                src_uv += dir * dist;
                if (src_uv.x < 0.0 || src_uv.x > 1.0 || src_uv.y < 0.0 || src_uv.y > 1.0)
                    break;
                if (dist < EPS)
                {
                    vec4 src_color = texture2D(al_tex, flip_y(src_uv));
                    radiance += src_color;
                    break;
                }
            }
        }
    }
    else if (length(light.rgb) > 0.1)
    {
        radiance = light;
    }

    radiance = max(light, radiance / float(num_rays));

    return vec4(pow(radiance.rgb, vec3(1. / POWER)), 1.0);
}

vec4 radiance_cascades()
{
    vec2 scaled_uv = flip_y(varying_texcoord);

    vec2 coord = floor(scaled_uv * bitmap_size);
    
    vec4 radiance = vec4(0.);

    float num_rays_rc = pow(base, cascade_index + 1.);
    float sqrt_base = sqrt(base);

    float delta_theta = 2. * PI / float(num_rays_rc);

    bool first_level = cascade_index == 0.;

    float spacing = pow(sqrt_base, cascade_index);

    vec2 size = floor(bitmap_size / spacing);
    vec2 probe_rel_pos = mod(coord, size);
    vec2 ray_pos = floor(coord / size);

    vec2 probe_center = (probe_rel_pos + 0.5) * spacing;
    vec2 normalized_probe_center = probe_center / bitmap_size;

    vec2 step_size = 1. / bitmap_size;
    float shortest_side = min(bitmap_size.x, bitmap_size.y);
    vec2 scale = shortest_side * step_size;

    float modifier_hack = 1.;

    float interval_start = first_level ? 0. : (modifier_hack * pow(base, cascade_index - 1.)) / shortest_side;
    float interval_length = (modifier_hack * pow(base, cascade_index)) / shortest_side;

    float base_index = float(base) * (ray_pos.x + (spacing * ray_pos.y));
    float min_step_size = min(step_size.x, step_size.y) * 0.5;

    for (int i = 0; i < int(base); i++)
    {
        float idx = base_index +  float(i);
        float theta_idx = idx + 0.5;
        float theta = delta_theta * theta_idx;

        vec2 dir = vec2(cos(theta), sin(theta));
        vec2 src_uv = normalized_probe_center + interval_start * dir * scale;

        bool dont_start = (src_uv.x < 0.0 || src_uv.x >= 1.0 || src_uv.y < 0.0 || src_uv.y >= 1.0);

        vec4 radiance_delta = vec4(0.);
        float traveled = 0.;

        for (int step = 1; step < num_steps && !dont_start; step++)
        {
            float dist = texture2D(distance_map, flip_y(src_uv)).r;
            src_uv += dir * dist * scale;

            if (src_uv.x < 0.0 || src_uv.x >= 1.0 || src_uv.y < 0.0 || src_uv.y >= 1.0)
                break;

            if (dist <= min_step_size)
            {
                vec4 src_color = texture2D(al_tex, flip_y(src_uv));
                radiance_delta += src_color;
                break;
            }
            
            traveled += dist;
            if (traveled >= interval_length)
                break;
        }

        bool non_opaque = radiance_delta.a == 0.;

        if (cascade_index < num_cascades - 1.0 && non_opaque)
        {
            float upper_spacing = pow(sqrt_base, cascade_index + 1.);
            vec2 upper_size = floor(bitmap_size / upper_spacing);
            vec2 upper_pos = vec2(mod(idx, upper_spacing), floor(idx / upper_spacing)) * upper_size;

            vec2 offt = (probe_rel_pos + 0.5) / sqrt_base;
            vec2 clamped = clamp(offt, vec2(0.5), upper_size - 0.5);

            vec4 upper_sample = texture2D(prev_cascade, flip_y((upper_pos + clamped) / bitmap_size));
            radiance_delta += upper_sample;
        }

        radiance += radiance_delta;
    }

    vec4 total_radiance = vec4(radiance.rgb / float(base), 1.);
    return vec4(last_index == 1 ? pow(total_radiance.rgb, vec3(1. / POWER)) : total_radiance.rgb, 1.0);
}

void main()
{
    gl_FragColor = radiance_cascades();
    //gl_FragColor = basic_raycasting();
}

