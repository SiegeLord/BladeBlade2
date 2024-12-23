uniform sampler2D al_tex;
varying vec4 varying_color;
varying vec2 varying_texcoord;
varying vec2 varying_pos;

uniform vec2 bitmap_size;

void main()
{
    vec4 color = varying_color * texture2D(al_tex, varying_texcoord);
    float mask = float(color.a > 0.);
    vec2 scaled_uv = mask * varying_pos / bitmap_size;
    gl_FragColor = vec4(scaled_uv, 0., 1.);
}

