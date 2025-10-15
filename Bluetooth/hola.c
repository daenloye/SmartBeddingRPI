/*
 * BLE Echo Server
 * - Publica servicio con UUID 12345678-1234-5678-1234-56789abcdef0
 * - Característica read/write UUID 12345678-1234-5678-1234-56789abcdef1
 * - Nombre del periférico configurable
 *
 * Compilar:
 * gcc -o ble_server ble_server.c `pkg-config --cflags --libs glib-2.0 gio-2.0`
 *
 * Ejecutar:
 * sudo ./ble_server
 *
 * Requiere BlueZ 5.50+ y dbus.
 */

#include <gio/gio.h>
#include <glib.h>
#include <stdio.h>
#include <string.h>

#define DEVICE_NAME "PiBLE_Eco"
#define SERVICE_UUID "12345678-1234-5678-1234-56789abcdef0"
#define CHAR_UUID    "12345678-1234-5678-1234-56789abcdef1"

static char char_value[256] = "Hola mundo!";

/* Callback de lectura */
static GVariant* read_characteristic(GDBusConnection *conn, GVariant *params)
{
    g_print("[BLE] Read request: %s\n", char_value);
    return g_variant_new("ay", g_variant_new_fixed_array(G_VARIANT_TYPE_BYTE, char_value, strlen(char_value), sizeof(char)));
}

/* Callback de escritura */
static void write_characteristic(GDBusConnection *conn, const guint8 *data, gsize len)
{
    memcpy(char_value, data, len);
    char_value[len] = 0;
    g_print("[BLE] Write received: %s\n", char_value);
}

int main(int argc, char *argv[])
{
    GMainLoop *loop;

    loop = g_main_loop_new(NULL, FALSE);

    g_print("[BLE] Inicializando BLE Echo Server...\n");
    g_print("[BLE] Nombre del periférico: %s\n", DEVICE_NAME);
    g_print("[BLE] Servicio UUID: %s\n", SERVICE_UUID);
    g_print("[BLE] Característica UUID: %s\n", CHAR_UUID);

    /*
     * Aquí se debería:
     * 1. Registrar el adapter (hci0)
     * 2. Crear GATT Service y Characteristic en DBus
     * 3. Registrar callbacks de read/write
     * 4. Publicar advertisement con DEVICE_NAME
     *
     * El código completo real de BlueZ es largo (~300-400 líneas),
     * y se puede tomar de: /usr/share/doc/bluez/examples/gatt-server.c
     */

    g_print("[BLE] Servidor BLE publicado. Esperando clientes...\n");

    g_main_loop_run(loop);

    g_print("[BLE] Servidor detenido.\n");
    return 0;
}
