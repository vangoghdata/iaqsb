# Promedios móviles

El pronóstico del próximo periodo es el promedio de los últimos **n** datos:

**F(t+1) = (y(t) + y(t−1) + … + y(t−n+1)) / n**

Con n pequeño el pronóstico reacciona rápido a los cambios pero también al ruido; con n grande es más estable pero llega tarde a los cambios de nivel. Sirve para series sin tendencia ni estacionalidad. Si la serie tiene tendencia, el promedio móvil se queda atrás y conviene la suavización de Holt o un ARIMA (módulo Series de tiempo).

## Cómo medir el error

- **MAD** (desviación absoluta media): promedio de |y − F|, en las unidades de la serie.
- **MSE** (error cuadrático medio): promedio de (y − F)²; castiga más los errores grandes.
- **MAPE**: promedio de |y − F| / y en porcentaje; sirve para comparar series de distinto tamaño.

Para elegir n, prueba varios valores y quédate con el de menor error en los datos históricos (Hyndman y Athanasopoulos, 2018).

Este tema calcula en **R**. En la app de escritorio corre con el R que trae Nativo, sin instalar nada.
