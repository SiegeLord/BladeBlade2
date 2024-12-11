uniform sampler2D al_tex;
varying vec4 varying_color;
varying vec2 varying_texcoord;

void main()
{
    gl_FragColor = varying_color * texture2D(al_tex, varying_texcoord);
}

