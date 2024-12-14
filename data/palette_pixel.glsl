uniform sampler2D al_tex;
varying vec4 varying_color;
varying vec2 varying_texcoord;

uniform float palette_index;
uniform sampler2D palette;

void main()
{
	float color_idx = texture2D(al_tex, varying_texcoord).r;
	vec4 color = texture2D(palette, vec2(color_idx, 1. - palette_index / 255.));
	//vec4 color = texture2D(palette, vec2(5. / 255., 1.));
	//vec4 color = vec4(1.);
	//vec4 color = vec4(color_idx * 32.);
    gl_FragColor = varying_color * color;
}

