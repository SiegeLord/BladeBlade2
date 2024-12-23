attribute vec4 al_pos;
attribute vec2 al_texcoord;
uniform mat4 al_projview_matrix;
varying vec2 varying_texcoord;
varying vec2 varying_pos;

void main()
{
   varying_texcoord = al_texcoord;
   gl_Position = al_projview_matrix * al_pos;
}

