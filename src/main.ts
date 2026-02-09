import { createApp } from 'vue'
import { createPinia } from 'pinia'
import router from './router'
import App from './App.vue'

// Import styles
import './assets/styles/main.css'

const app = createApp(App)

// State management
app.use(createPinia())

// Routing
app.use(router)

app.mount('#app')
