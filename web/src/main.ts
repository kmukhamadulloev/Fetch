import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import router from './router'
import { initializeAppearance } from './stores/appearance'
import './styles.css'

initializeAppearance()
createApp(App).use(createPinia()).use(router).mount('#app')
