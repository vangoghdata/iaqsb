# La mezcla de productos

Una planta fabrica varios productos que compiten por los mismos recursos: horas de máquina, horas de mano de obra, materiales. La **mezcla de productos** decide cuánto fabricar de cada uno para ganar lo máximo sin pasarse de lo disponible. Es el problema con el que nació la programación lineal y sigue siendo el corazón de la planeación de producción.

## El modelo

- **Variables:** xₚ = unidades que se fabrican del producto p.
- **Objetivo:** maximizar la utilidad total Σ uₚ · xₚ.
- **Restricciones:** para cada recurso, Σ (horas por unidad) · xₚ ≤ horas disponibles; además, no fabricar más de lo que se puede vender (xₚ ≤ demanda máxima) y xₚ ≥ 0.

## Cómo leer la solución

- **Valor de cada variable:** cuánto fabricar de cada producto.
- **Precio sombra de un recurso:** cuánto aumentaría la utilidad con una hora más de ese recurso. Si es cero, el recurso sobra; si es alto, ese recurso es el cuello de botella y vale la pena ampliarlo (horas extra, otro turno) mientras cueste menos que su precio sombra.
- **Costo reducido de un producto:** cuánto tendría que mejorar su utilidad para que convenga fabricarlo, si la solución no lo incluye.

## Para practicar

Cambia las horas disponibles y vuelve a resolver: observa cómo cambia la mezcla y cuándo un recurso deja de ser el cuello de botella. El modelo completo, escrito en el lenguaje algebraico de Nativo, está en el archivo `modelo.iaqsb` del tema; lo puedes abrir en el módulo de programación lineal para estudiarlo.
