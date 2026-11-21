import { createApp } from 'vue'
import { createPinia } from 'pinia'
import '@muyajs/core/assets/styles/index.css'
import './style.css'
import { installTauriMark5Bridge } from './api/tauriBridge'
import App from './App.vue'

installTauriMark5Bridge()

const app = createApp(App)
app.use(createPinia())
app.mount('#app')
