# Cómo se usa: un pedido de punta a punta

Este es el recorrido de un pedido en cualquier ERP (los nombres de las pantallas cambian entre SAP, Dynamics u Odoo, pero la lógica es la misma). Para practicarlo, instala una versión de prueba de Odoo o ERPNext con datos de ejemplo y sigue los pasos.

1. **Datos maestros.** Se crean el producto terminado, sus componentes, la **lista de materiales** (qué lleva y cuánto) y la **ruta** (en qué centros de trabajo se hace y cuánto tarda). También el cliente y los proveedores con sus tiempos de entrega.
2. **Pedido de venta.** El vendedor registra el pedido: cliente, producto, cantidad, precio y fecha prometida. El ERP revisa si hay existencias o si hay que fabricar.
3. **Planeación (MRP).** Se corre el MRP: el sistema explota la lista de materiales, descuenta el inventario y las órdenes en curso, y propone **órdenes de fabricación** y **órdenes de compra** con sus fechas (el mismo cálculo que hace el módulo MRP de Nativo).
4. **Compras.** El comprador confirma las órdenes de compra sugeridas. Cuando llega el material, la bodega registra la **recepción** y el inventario sube.
5. **Producción.** Se libera la orden de fabricación: el sistema descuenta los componentes del inventario y, al terminar, suma el producto terminado. Se registran las horas y los desperdicios.
6. **Despacho.** La bodega prepara y despacha el pedido; el inventario de producto terminado baja.
7. **Facturación y cobro.** Se emite la factura (electrónica, en Colombia ante la DIAN) y, cuando el cliente paga, se registra el cobro.
8. **Contabilidad y costos.** Cada paso anterior generó su asiento contable sin que nadie lo digitara: así se conoce el costo real del pedido y su margen.

## Qué mirar después

- **Cumplimiento de pedidos:** cuántos se entregaron completos y a tiempo (el módulo de indicadores logísticos de Nativo lo calcula).
- **Exactitud del inventario:** lo que dice el sistema frente a lo que hay en la bodega.
- **Excepciones del MRP:** órdenes vencidas o reprogramadas; muchas excepciones indican datos maestros malos o un programa maestro inestable.

## Para practicar en Nativo

- El módulo **MRP** hace la explosión de materiales del paso 3 con sus reglas de lote.
- El **Programa maestro** decide cuánto fabricar de cada producto terminado.
- Las **Finanzas del inventario** muestran cuánto cuesta el inventario que el ERP mantiene.
