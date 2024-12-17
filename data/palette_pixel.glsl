uniform sampler2D al_tex;
varying vec4 varying_color;
varying vec2 varying_texcoord;
varying vec2 varying_material;
varying vec4 varying_pos;

uniform sampler2D palette;
uniform bool use_texture;
uniform float show_depth;

void main()
{
	float color_idx = texture2D(al_tex, varying_texcoord).r;
	float palette_index = varying_material.x;
	if (color_idx == 0.0)
		discard;
	vec4 color = texture2D(palette, vec2(color_idx, 1. - palette_index / 255.));
	//vec4 color = texture2D(palette, vec2(5. / 255., 1.));
	//vec4 color = vec4(1.);
	//vec4 color = vec4(color_idx * 32.);
	vec4 depth_color = 0.5 + 0.5 * vec4(varying_pos.z, varying_pos.z, varying_pos.z, 1.);
	gl_FragColor = varying_color * ((1. - show_depth) * color + show_depth * depth_color);
}

