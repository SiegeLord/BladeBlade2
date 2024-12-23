uniform sampler2D al_tex;
varying vec2 varying_texcoord;

uniform vec2 bitmap_size;
uniform float uv_offset;

vec2 flip_y(vec2 uv)
{
    return vec2(uv.x, 1. - uv.y);
}

void main()
{
    vec4 nearest_seed = vec4(0.0);
    float nearest_dist = 10000000.;

    vec2 scaled_uv = flip_y(varying_texcoord);

    for (float y = -1.0; y <= 1.0; y += 1.0)
    {
        for (float x = -1.0; x <= 1.0; x += 1.0)
        {
           vec2 src_uv = scaled_uv + vec2(x, y) * uv_offset / bitmap_size;

           if (src_uv.x < 0.0 || src_uv.x > 1.0 || src_uv.y < 0.0 || src_uv.y > 1.0)
	   {
	       continue;
	   }

           vec2 cand_seed = texture2D(al_tex, flip_y(src_uv)).xy;
           if (cand_seed.x != 0.0 || cand_seed.y != 0.0)
	   {
               vec2 diff = cand_seed - scaled_uv;
               float dist = dot(diff, diff);
               if (dist < nearest_dist)
	       {
                   nearest_dist = dist;
                   nearest_seed.xy = cand_seed.xy;
               }
           }
        }
    }
    gl_FragColor = nearest_seed;
}
