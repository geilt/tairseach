import { createApp } from 'vue'
import { createPinia } from 'pinia'
import router from './router'
import App from './App.vue'
import { showToast } from './composables/useToast'

import './assets/styles/main.css'

const app = createApp(App)

app.config.errorHandler = (err, _instance, info) => {
  console.error('[Global Vue Error]', err, info)
  showToast.error(`Unexpected error: ${info}`)
  // TODO: beacon to backend error_report command when available.
}

app.use(createPinia())
app.use(router)
app.mount('#app')
