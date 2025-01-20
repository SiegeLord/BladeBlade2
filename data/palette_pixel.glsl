uniform sampler2D al_tex;
varying vec4 varying_color;
varying vec2 varying_texcoord;
varying vec2 varying_material;
varying vec4 varying_pos;

uniform sampler2D palette;
uniform bool use_texture;
uniform float show_depth;
uniform vec2 bitmap_size;

uniform sampler2D light;

void main()
{
	float color_idx = texture2D(al_tex, varying_texcoord).r;
	float palette_index = varying_material.x;
	float material = varying_material.y;
	if (color_idx == 0.0)
		discard;
	vec4 color = texture2D(palette, vec2(color_idx, 1. - palette_index / 255.));

	if (material == 1.0)
	{
		float b = 0.30 * color.r + 0.59 * color.g + 0.11 * color.b;
		color = vec4(0.1 * b, 0.1 * b, b, color.a);
	}

	vec4 light_color = vec4(1.);
	if (material == 2.0)
	{
		light_color = 4. * texture2D(light, 0.5 * varying_pos.xy + vec2(0.5, 0.5));
		light_color = vec4(0.4) + light_color * 0.6;
		light_color = vec4(light_color.rgb, 1.);
	}

	if (material == 3.0)
	{
		light_color = vec4(vec3(0.6), 1.);
	}

	//vec4 color = texture2D(palette, vec2(5. / 255., 1.));
	//vec4 color = vec4(1.);
	//vec4 color = vec4(color_idx * 32.);
	vec4 depth_color = 0.5 + 0.5 * vec4(varying_pos.z, varying_pos.z, varying_pos.z, 1.);
	gl_FragColor = light_color * varying_color * ((1. - show_depth) * color + show_depth * depth_color);
	//gl_FragColor = vec4(varying_pos.xy, 0., 1.);
}

