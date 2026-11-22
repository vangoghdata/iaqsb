// Al entrar a una celda numérica se selecciona todo su contenido: lo que se
// escriba reemplaza el valor (un 0 que trae la hoja se vuelve 4, no 40).

function esNumerica(el: EventTarget | null): el is HTMLInputElement {
  if (!(el instanceof HTMLInputElement)) return false;
  if (el.readOnly || el.disabled) return false;
  if (el.type === 'number') return true;
  if (el.type !== 'text' && el.type !== '') return false;
  return el.inputMode === 'decimal' || el.inputMode === 'numeric' || el.classList.contains('num') || el.classList.contains('cell') || el.hasAttribute('data-r');
}

export function seleccionarAlEntrar() {
  // El clic que da el foco no debe deshacer la selección al soltar el botón.
  let recienEnfocado: HTMLInputElement | null = null;

  document.addEventListener('focusin', (e) => {
    const el = e.target;
    if (!esNumerica(el)) return;
    recienEnfocado = el;
    // después de que el navegador ponga el cursor
    requestAnimationFrame(() => {
      if (document.activeElement === el) el.select();
    });
  });

  document.addEventListener('mouseup', (e) => {
    if (recienEnfocado && e.target === recienEnfocado) {
      e.preventDefault();
      if (recienEnfocado.selectionStart === recienEnfocado.selectionEnd) recienEnfocado.select();
    }
    recienEnfocado = null;
  });
}
