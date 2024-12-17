uniform sampler2D al_tex;
uniform bool al_use_tex;
varying vec4 varying_color;
varying vec2 varying_texcoord;

void main()
{
    vec4 color;
    if (al_use_tex)
	color = varying_color * texture2D(al_tex, varying_texcoord);
    else
	color = varying_color;
    gl_FragColor = color;
}

