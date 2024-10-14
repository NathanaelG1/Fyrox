(
    name: "StandardSpriteShader",

    resources: [
        (
            name: "diffuseTexture",
            kind: Texture(default: None, kind: Sampler2D, fallback: White),
            binding: 0
        ),
        (
            name: "fyrox_instanceData",
            kind: PropertyGroup([
                // Autogenerated
            ]),
            binding: 0
        ),
        (
            name: "fyrox_cameraData",
            kind: PropertyGroup([
                // Autogenerated
            ]),
            binding: 1
        ),
    ],

    passes: [
        (
            name: "Forward",
            draw_parameters: DrawParameters(
                cull_face: None,
                color_write: ColorMask(
                    red: true,
                    green: true,
                    blue: true,
                    alpha: true,
                ),
                depth_write: false,
                stencil_test: None,
                depth_test: Some(Less),
                blend: Some(BlendParameters(
                    func: BlendFunc(
                        sfactor: SrcAlpha,
                        dfactor: OneMinusSrcAlpha,
                        alpha_sfactor: SrcAlpha,
                        alpha_dfactor: OneMinusSrcAlpha,
                    ),
                    equation: BlendEquation(
                        rgb: Add,
                        alpha: Add
                    )
                )),
                stencil_op: StencilOp(
                    fail: Keep,
                    zfail: Keep,
                    zpass: Keep,
                    write_mask: 0xFFFF_FFFF,
                ),
                scissor_box: None
            ),
            vertex_shader:
               r#"
                layout(location = 0) in vec3 vertexPosition;
                layout(location = 1) in vec2 vertexTexCoord;
                layout(location = 2) in vec2 vertexParams;
                layout(location = 3) in vec4 vertexColor;

                out vec2 texCoord;
                out vec4 color;

                void main()
                {
                    float size = vertexParams.x;
                    float rotation = vertexParams.y;

                    texCoord = vertexTexCoord;
                    color = vertexColor;
                    vec2 vertexOffset = S_RotateVec2(vertexTexCoord * 2.0 - 1.0, rotation);
                    vec4 worldPosition = fyrox_instanceData.worldMatrix * vec4(vertexPosition, 1.0);
                    vec3 offset = (vertexOffset.x * fyrox_cameraData.sideVector + vertexOffset.y * fyrox_cameraData.upVector) * size;
                    gl_Position = fyrox_cameraData.viewProjectionMatrix * (worldPosition + vec4(offset.x, offset.y, offset.z, 0.0));
                }
               "#,

           fragment_shader:
               r#"
                out vec4 FragColor;

                in vec2 texCoord;
                in vec4 color;

                void main()
                {
                    FragColor = color * S_SRGBToLinear(texture(diffuseTexture, texCoord));
                }
               "#,
        )
    ],
)