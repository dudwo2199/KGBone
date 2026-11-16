// KGBone 런타임 셰이더(URP): 미리 곱한 알파 텍스처 × 정점 색 + 어두운 색(tint black).
// 색 공식은 KGBone 화면(app/src/render/gl.ts)과 같다: rgb = (t.rgb × light.rgb + (t.a − t.rgb) × dark) × light.a, a = t.a × light.a
// 블렌드 함수는 머티리얼 속성(_SrcBlend, _DstBlend)으로 정한다(app/src/render/blend.ts의 4종).
Shader "KGBone/Unlit"
{
    Properties
    {
        _MainTex ("Texture", 2D) = "white" {}
        [HideInInspector] _SrcBlend ("Src Blend", Float) = 1
        [HideInInspector] _DstBlend ("Dst Blend", Float) = 10
    }
    SubShader
    {
        Tags { "RenderPipeline" = "UniversalPipeline" "Queue" = "Transparent" "RenderType" = "Transparent" "IgnoreProjector" = "True" }
        Pass
        {
            Name "KGBoneUnlit"
            Blend [_SrcBlend] [_DstBlend]
            ZWrite Off
            Cull Off

            HLSLPROGRAM
            #pragma vertex vert
            #pragma fragment frag
            #include "Packages/com.unity.render-pipelines.universal/ShaderLibrary/Core.hlsl"

            TEXTURE2D(_MainTex);
            SAMPLER(sampler_MainTex);

            CBUFFER_START(UnityPerMaterial)
                float4 _MainTex_ST;
                float _SrcBlend;
                float _DstBlend;
            CBUFFER_END

            struct Attributes
            {
                float3 positionOS : POSITION;
                float4 color : COLOR;
                float2 uv : TEXCOORD0;
                float4 dark : TEXCOORD1;
            };

            struct Varyings
            {
                float4 positionCS : SV_POSITION;
                float4 color : COLOR;
                float2 uv : TEXCOORD0;
                float3 dark : TEXCOORD1;
            };

            // 표준 sRGB → 선형 변환(구간별 공식)
            half3 KGBoneSrgbToLinear(half3 c)
            {
                half3 lo = c / 12.92;
                half3 hi = pow(max((c + 0.055) / 1.055, 0.0), 2.4);
                return lerp(hi, lo, step(c, 0.04045));
            }

            Varyings vert(Attributes i)
            {
                Varyings o;
                o.positionCS = TransformObjectToHClip(i.positionOS);
                o.color = i.color;
                o.uv = i.uv;
                o.dark = i.dark.rgb;
                return o;
            }

            half4 frag(Varyings i) : SV_Target
            {
                half4 t = SAMPLE_TEXTURE2D(_MainTex, sampler_MainTex, i.uv);
                // 텍스처·정점 색은 감마 공간 값 그대로다(텍스처는 sRGB로 풀지 않게 만든다). 화면과 같은 식으로 계산한다
                half k = i.color.a;
                half3 rgb = (t.rgb * i.color.rgb + (t.a - t.rgb) * i.dark) * k;
                half a = t.a * k;
            #if !defined(UNITY_COLORSPACE_GAMMA)
                // 선형 색 공간 프로젝트: 결과 색을 선형으로 바꿔 내보낸다(미리 곱한 알파를 풀었다가 다시 곱함).
                // 불투명한 부분은 화면과 같은 색이 되고, 반투명 가장자리와 블렌드는 선형 공간에서 섞여 조금 다를 수 있다
                half3 straight = a > 0.0001 ? rgb / a : half3(0, 0, 0);
                rgb = KGBoneSrgbToLinear(straight) * a;
            #endif
                return half4(rgb, a);
            }
            ENDHLSL
        }
    }
}
