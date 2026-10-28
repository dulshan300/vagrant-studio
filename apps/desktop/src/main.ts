import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import { createAppRouter } from './app/router'
import { useSettingsStore } from './stores/settings'
import './styles/main.css'

const app = createApp(App)
app.use(createPinia())
app.use(createAppRouter())

// Apply the theme before the first render to avoid a light/dark flash.
useSettingsStore()

app.mount('#app')
