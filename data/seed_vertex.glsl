attribute vec4 al_pos;
attribute vec4 al_color;
attribute vec2 al_texcoord;
uniform mat4 al_projview_matrix;
varying vec4 varying_color;
varying vec2 varying_texcoord;
varying vec2 varying_pos;

void main()
{
   varying_color = al_color;
   varying_texcoord = al_texcoord;
   varying_pos = al_pos.xy;
   gl_Position = al_projview_matrix * al_pos;
}

