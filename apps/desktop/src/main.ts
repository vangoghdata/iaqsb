import { mount } from 'svelte';
import './app.css';
import App from './App.svelte';
import { seleccionarAlEntrar } from './lib/seleccion';

seleccionarAlEntrar();

const app = mount(App, { target: document.getElementById('app')! });

export default app;
