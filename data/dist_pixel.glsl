uniform sampler2D al_tex;
varying vec2 varying_texcoord;

vec2 flip_y(vec2 uv)
{
    return vec2(uv.x, 1. - uv.y);
}

void main()
{
    vec2 scaled_uv = flip_y(varying_texcoord);
    vec2 nearest_seed = texture2D(al_tex, flip_y(scaled_uv)).xy;
    float dist = clamp(distance(scaled_uv, nearest_seed), 0., 1.);
    gl_FragColor = vec4(vec3(dist), 1.);
}

