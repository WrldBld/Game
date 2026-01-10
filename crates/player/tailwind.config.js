/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./src/**/*.rs",
    "../player-runner/index.html",
  ],
  theme: {
    extend: {
      colors: {
        // Dark UI theme colors
        'dark': {
          'bg': '#0f0f23',
          'surface': '#1a1a2e',
          'border': '#374151',
          'hover': '#252540',
          'gradient-end': '#16213e',
          'purple-end': '#2d1b3d',
        },
        // TTRPG themed colors
        'parchment': {
          50: '#fdfcf9',
          100: '#f9f5eb',
          200: '#f3ead5',
          300: '#e9d9b5',
          400: '#dcc48f',
          500: '#cfae6a',
          600: '#b8914d',
          700: '#9a7540',
          800: '#7d5f38',
          900: '#664e31',
        },
        'ink': {
          50: '#f6f6f7',
          100: '#e2e3e5',
          200: '#c5c6cb',
          300: '#a1a3ab',
          400: '#7d808a',
          500: '#62656f',
          600: '#4d4f58',
          700: '#3f4148',
          800: '#35373c',
          900: '#2e3034',
        },
        'blood': {
          50: '#fef2f2',
          100: '#fee2e2',
          200: '#fecaca',
          300: '#fca5a5',
          400: '#f87171',
          500: '#8b0000',
          600: '#7c0000',
          700: '#6b0000',
          800: '#5a0000',
          900: '#4a0000',
        },
        'gold': {
          50: '#fffbeb',
          100: '#fef3c7',
          200: '#fde68a',
          300: '#fcd34d',
          400: '#fbbf24',
          500: '#d4af37',
          600: '#b8972f',
          700: '#9a7e27',
          800: '#7c661f',
          900: '#5e4d17',
        },
      },
      fontFamily: {
        'fantasy': ['Cinzel', 'Georgia', 'serif'],
        'body': ['Lora', 'Georgia', 'serif'],
        'ui': ['Inter', 'system-ui', 'sans-serif'],
      },
      backgroundImage: {
        'parchment-texture': "url('/assets/textures/parchment.png')",
        'leather-texture': "url('/assets/textures/leather.png')",
      },
      boxShadow: {
        'vignette': 'inset 0 0 100px rgba(0,0,0,0.3)',
        'glow': '0 0 20px rgba(212, 175, 55, 0.5)',
      },
      animation: {
        'typewriter': 'typewriter 2s steps(40) forwards',
        'fade-in': 'fadeIn 0.5s ease-in-out',
        'fade-out': 'fadeOut 0.5s ease-in-out forwards',
        'slide-up': 'slideUp 0.3s ease-out',
        'backdrop-crossfade': 'backdropCrossfade 0.8s ease-in-out',
      },
      keyframes: {
        typewriter: {
          'from': { width: '0' },
          'to': { width: '100%' },
        },
        fadeIn: {
          'from': { opacity: '0' },
          'to': { opacity: '1' },
        },
        fadeOut: {
          'from': { opacity: '1' },
          'to': { opacity: '0' },
        },
        slideUp: {
          'from': { transform: 'translateY(20px)', opacity: '0' },
          'to': { transform: 'translateY(0)', opacity: '1' },
        },
        backdropCrossfade: {
          '0%': { opacity: '0' },
          '50%': { opacity: '1' },
          '100%': { opacity: '0' },
        },
      },
    },
  },
  plugins: [],
}
