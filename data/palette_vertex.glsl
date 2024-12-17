attribute vec4 al_pos;
attribute vec4 al_color;
attribute vec2 al_texcoord;
attribute vec2 al_user_attr_0; // Material.
uniform mat4 al_projview_matrix;
varying vec4 varying_color;
varying vec2 varying_texcoord;
varying vec2 varying_material;
varying vec4 varying_pos;

void main()
{
   varying_color = al_color;
   varying_texcoord = al_texcoord;
   varying_material = al_user_attr_0;
   varying_pos = al_projview_matrix * al_pos;
   gl_Position = al_projview_matrix * al_pos;
}

