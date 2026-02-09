/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{vue,js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        naonur: {
          // Primary
          gold: {
            DEFAULT: '#C9A227',
            dim: '#8B7019',
            bright: '#E5C363',
          },
          // Backgrounds
          void: '#0A0A0F',
          shadow: '#12121A',
          mist: '#1A1A24',
          fog: '#252532',
          cloud: '#32323F',
          // Text
          bone: '#E8E4D9',
          ash: '#9A978F',
          smoke: '#5A5850',
          // Status
          moss: {
            DEFAULT: '#4A7C59',
            dim: '#3A5C45',
          },
          rust: {
            DEFAULT: '#8B4513',
            dim: '#6B3510',
          },
          blood: {
            DEFAULT: '#8B0000',
            dim: '#6B0000',
          },
          water: {
            DEFAULT: '#4A7C8C',
            dim: '#3A5C6C',
          },
        },
      },
      fontFamily: {
        display: ['Cinzel', 'serif'],
        body: ['Cormorant Garamond', 'serif'],
        mono: ['JetBrains Mono', 'monospace'],
      },
      boxShadow: {
        'glow': '0 0 20px rgba(201, 162, 39, 0.3)',
        'glow-sm': '0 0 10px rgba(201, 162, 39, 0.2)',
      },
      animation: {
        'fade-in': 'fadeIn 250ms ease-out',
        'slide-in': 'slideInRight 250ms ease-out',
        'slide-up': 'slideInUp 250ms ease-out',
      },
      keyframes: {
        fadeIn: {
          from: { opacity: '0' },
          to: { opacity: '1' },
        },
        slideInRight: {
          from: { transform: 'translateX(20px)', opacity: '0' },
          to: { transform: 'translateX(0)', opacity: '1' },
        },
        slideInUp: {
          from: { transform: 'translateY(10px)', opacity: '0' },
          to: { transform: 'translateY(0)', opacity: '1' },
        },
      },
    },
  },
  plugins: [],
}
