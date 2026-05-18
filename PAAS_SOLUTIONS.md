# 🚀 CodeCraft Perú — Portafolio de Soluciones PaaS Industriales

Este documento constituye el **plano técnico y comercial** de los productos de software de CodeCraft Perú. Ha sido diseñado para servir como manual de consulta recurrente para desarrollo de software, auditorías y propuestas comerciales B2B.

---

## 🟢 1. WeighFlow IoT
*Telemetría y Sincronización Blindada de Balanzas Industriales*

*   **Problema Crítico:** Fraudes operativos de "peso fantasma", manipulación manual de registros de peso de concentrado de mineral, y tiempos excesivos de pesaje (cuellos de botella).
*   **¿Cómo funciona?**
    1.  Un agente nativo ligero escrito en **Rust** se conecta al puerto físico RS-232/TCP-IP del cabezal indicador de la balanza (ej. Mettler Toledo, Rinstrum).
    2.  El agente intercepta e interpreta las tramas de bytes continuas (ej. formato STX/ETX con suma de verificación).
    3.  Añade un **sello criptográfico digital (HMAC/AES-256)** que vincula el peso exacto con la hora y el tag NFC del camión.
    4.  Transmite los datos a un backend en **Spring Boot** que almacena el registro en base de datos.
    5.  El operador visualiza el ticket en tiempo real en un dashboard en **Angular** mediante WebSockets.
*   **Especificaciones Tecnológicas:**
    *   **Edge:** Rust (latencia sub-milisegundo, cero recolección de basura).
    *   **Backend:** Java 21, Spring Boot (Spring WebFlux reactivo), PostgreSQL.
    *   **Frontend:** Angular 17+, WebSockets nativos.
    *   **Protocolos:** RS-232, TCP/IP, STX/CKS.
*   **Propuesta de Valor B2B:** Reduce el tiempo de pesaje de 4 minutos a solo 12 segundos y erradica el fraude por peso manipulado al 100%.

---

## 🔵 2. VentiFlow IoT
*Optimización Dinámica de Ventilación en Mina Subterránea (VoD)*

*   **Problema Crítico:** Altísimo consumo de energía eléctrica por extractores operando al 100% de potencia continuamente, y riesgo de asfixia/intoxicación por acumulación de gases de maquinaria diésel.
*   **¿Cómo funciona?**
    1.  Módulos IoT capturan continuamente lecturas de calidad de aire ($CO, NO_2, O_2$, polvo en suspensión) desde sensores distribuidos en los túneles.
    2.  Lectores RFID en los frentes de trabajo rastrean el posicionamiento de maquinaria pesada y personal.
    3.  El motor de toma de decisiones en la nube calcula dinámicamente el caudal de aire óptimo según regulaciones de seguridad.
    4.  Envía comandos automáticos mediante controladores PLC para ajustar la velocidad de los variadores de frecuencia de los extractores gigantes de aire.
    5.  El centro de control visualiza la concentración de gases en un mapa SVG interactivo en **Angular**.
*   **Especificaciones Tecnológicas:**
    *   **Edge/Gateway:** Rust (filtrado en el edge y resiliencia sin internet).
    *   **Industrial:** PLCs, Modbus/TCP, OPC UA.
    *   **Cloud:** Microservicios reactivos en Spring WebFlux.
    *   **Frontend:** Angular (visualización SVG interactiva y WebSockets).
*   **Propuesta de Valor B2B:** Ahorro comprobado de hasta un 40% en facturación eléctrica mensual y control automatizado de riesgos de seguridad y salud ocupacional (EHS).

---

## 🟠 3. LoadGrade PaaS
*Sistema de Despacho y Prevención de Mezcla de Mineral*

*   **Problema Crítico:** Mezcla accidental de mineral de alta ley con desmonte en chancadoras o botaderos, representando pérdidas de millones de dólares.
*   **¿Cómo funciona?**
    1.  Al cargar el volquete en el tajo, la pala escanea el tag NFC/QR asignándole una categoría de mineral (Alta Ley, Media Ley o Desmonte) y un destino.
    2.  El chofer visualiza su ruta y alertas sonoras guiadas mediante una aplicación robusta en **Kotlin Nativo** montada en la cabina del camión.
    3.  El sistema monitorea la coordenada GPS diferencial del camión en tiempo real con precisión centimétrica.
    4.  Si el camión con desmonte intenta descargar en la chancadora de mineral, la nube detecta la violación de geocerca en milisegundos, activa una alarma sonora masiva en cabina, bloquea la báscula de descarga y reporta al despachador central.
*   **Especificaciones Tecnológicas:**
    *   **Edge/Tablet:** Kotlin Nativo (Android Rugged Devices), SQLite (diseño Offline-First).
    *   **Backend:** Spring Boot (algoritmos basados en teoría de grafos para ruteo de flotas).
    *   **GIS Database:** PostgreSQL con extensión PostGIS para geocercas en tiempo real.
    *   **Hardware:** GPS diferencial RTK de precisión centimétrica.
*   **Propuesta de Valor B2B:** Cero pérdidas financieras por mezcla de mineral de ley con desmonte y mejora en la eficiencia de transporte de flota en un 15%.

---

## 🔴 4. TireTrack IoT
*Telemetría Térmica Predictiva de Neumáticos de Alto Costo OTR*

*   **Problema Crítico:** El altísimo costo de neumáticos mineros gigantescos (OTR) que pueden reventar catastróficamente debido a exceso de calor (TKPH) o presión inadecuada durante el acarreo.
*   **¿Cómo funciona?**
    1.  Sensores RF integrados en las válvulas de las llantas miden continuamente presión y temperatura interior.
    2.  Una pasarela IoT a bordo del camión (escrita en **Rust**) recopila los paquetes RF, filtrando ruidos electromagnéticos del chasis.
    3.  Los datos viajan por celular/Wi-Fi en malla hacia un backend de alto flujo gestionado en **Java 21/Spring Boot** con **RabbitMQ**.
    4.  Las lecturas temporales se almacenan en una base de datos **TimescaleDB** optimizada para series de tiempo.
    5.  Algoritmos predictivos analizan las tendencias térmicas; si un neumático supera su límite de calor por hora (TKPH), se emite una alerta temprana en Angular para desviar temporalmente el camión a enfriamiento.
*   **Especificaciones Tecnológicas:**
    *   **Edge:** Gateway embebido en Rust con interfaces RF y CAN bus.
    *   **Ingesta:** Java 21, Spring Boot, Spring AMQP (RabbitMQ).
    *   **Base de datos:** TimescaleDB (PostgreSQL optimizado para series de tiempo).
    *   **Frontend:** Angular (gráficos vectoriales de alta densidad de datos en tiempo real).
*   **Propuesta de Valor B2B:** Aumenta la vida útil de los costosos neumáticos OTR en un 25% y elimina accidentes fatales por explosión de llantas de alta presión.
