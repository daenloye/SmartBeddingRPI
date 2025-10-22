import numpy as np
import os
from scipy.stats import  kurtosis
import ast
from joblib import load
from skimage.metrics import structural_similarity as ssim

#Funciones

def estandarizar_matriz(matriz):
        # Convertir matriz a tipo entero si no lo está
        matriz = np.array(matriz, dtype=int)

        # Calcular promedio y desviación estándar
        promedio = np.mean(matriz)
        desv = np.std(matriz)

        # Aplicar z-score
        matriz = (matriz - promedio) / desv

        # Hacer los valores positivos sumando el valor mínimo
        min_value = np.min(matriz)
        matriz -= min_value

        # Calcular el tercer cuartil (Q3)
        q3 = np.percentile(matriz, 75)

        # matriz=interpolar_mapa(matriz)

        # Añadir al dataset estandarizado
        return matriz

def flipDato(dato):
  return np.flip(np.flip(dato, axis=1), axis=0)

def clipDato(dato):
  return np.clip(dato, 0, np.percentile(dato, 75))

def kurystd(imagen):
  return kurtosis(imagen.flatten()),np.std(imagen)

def mascara_binaria_manual(imagen,umbral):
  return np.where(imagen > umbral, 255, 0).astype(np.uint8)

def calcular_area_blanca_manual(imagen,umbral=None):

    if umbral==None:
      #Se calcula el umbral automaticamente
      umbral=np.max(imagen)*150/255

    #Crear una máscara binaria donde los valores > umbral se conviertan en 255
    mascara_blanca = mascara_binaria_manual(imagen, umbral)

    #Verificar si la máscara tiene datos válidos
    if mascara_blanca is None or mascara_blanca.size == 0:
        return 0

    #Calcular el área blanca
    area_blanca = np.sum(mascara_blanca) / 255  # Dividir entre 255 para contar píxeles blancos

    return area_blanca

def calcular_area_blanca_regiones_manual(imagen, umbral=None, factor_interpolacion=1):
    if umbral is None:
        umbral = np.max(imagen) * 150 / 255

    areas = np.zeros((2, 3))
    rows, cols = imagen.shape

    forRow = int(rows / 2)
    forCol = int((cols - 1) / 3)

    for fila in range(2):
        for columna in range(3):
            pedazo = imagen[forRow*fila:forRow*(fila+1), forCol*columna:forCol*(columna+1)]
            areas[fila, columna] = calcular_area_blanca_manual(pedazo, umbral)

    # 🔧 Ajuste por interpolación (corrección del área)
    if factor_interpolacion != 1:
        areas = areas / (factor_interpolacion ** 2)

    return areas

def calcular_simetria(imagen):
    rows, cols = imagen.shape
    mid = cols // 2

    # Dividir la imagen
    lizq = imagen[:, :mid]
    lder = imagen[:, mid:]

    # Asegurar mismo tamaño (por si es impar o hay recorte irregular)
    min_cols = min(lizq.shape[1], lder.shape[1])
    lizq = lizq[:, :min_cols]
    lder = lder[:, :min_cols]

    # Invertir la mitad derecha para comparar en espejo
    lder_inv = lder[:, ::-1]

    # 🧩 Asegurar que la imagen sea suficientemente grande para ssim()
    h, w = lizq.shape
    win_size = min(7, h, w)
    if win_size % 2 == 0:  # debe ser impar
        win_size -= 1
    if win_size < 3:
        # Si es demasiado pequeña, devolvemos 0 o NaN en lugar de lanzar error
        return 0

    # Calcular simetría con win_size seguro
    sim_ski = ssim(lizq, lder_inv, win_size=win_size, data_range=lder.max() - lder.min())

    return sim_ski

def calcular_balance(imagen):

  #Se calcula el umbral automaticamente
  umbral=np.max(imagen)*150/255

  #Aplico la mascara
  mascara=mascara_binaria_manual(imagen, umbral)

  #Obtengo las dimensiones de la imagen
  rows,cols=imagen.shape

  #Obtengo las columnas por division
  columnas=int(cols/2)

  #Parto la imagen
  lizq=mascara[:,0:columnas]
  lder=mascara[:,columnas:]

  #Calcula coverage
  coverage_izquierda = np.sum(lizq) / 255 #se divide por 255 para obtener el número total de píxeles blancos en lugar del valor acumulado de intensidades.
  coverage_derecha = np.sum(lder) / 255 #se divide por 255 para obtener el número total de píxeles blancos en lugar del valor acumulado de intensidades.

  if (coverage_izquierda - coverage_derecha) == 0:
      return 0  # Otra opción podría ser devolver None o un valor que tenga sentido en tu contexto
  else:
      balance = (coverage_izquierda - coverage_derecha) / (coverage_izquierda + coverage_derecha)  # Valor entre -1 y 1
      return balance

def centro_de_masa(matriz):
    # Crear arreglos de coordenadas para filas (y) y columnas (x)
    y_coords, x_coords = np.indices(matriz.shape)

    # Calcular el centro de masa basado en los valores reales de la matriz
    total_intensidad = np.sum(matriz)
    if total_intensidad == 0:
        raise ValueError("La matriz no contiene elementos activos (todos son 0)")

    # Calcular las coordenadas del centro de masa
    x_c = np.sum(x_coords * matriz) / total_intensidad
    y_c = np.sum(y_coords * matriz) / total_intensidad

    return (x_c, y_c)  # Devolvemos en orden (fila, columna)

def procesarMuestra(data_raw,side="R"):
    data = np.array(data_raw)

    #Analizo valores para determinar si hay alguien o no
    min=np.min(data)
    max=np.max(data)
    desv=np.std(data)
    vari=np.var(data)

    #Analizo si la sábana está vacía
    if max>5000 or desv>700 or vari>60000:
        return -1


    # Calculo kurtosis y std
    kurtosis, std = kurystd(data)

    # Ahora estandarizo, reflejo y hago clip
    estandarizado = estandarizar_matriz(data)
    clipeado = clipDato(estandarizado)

    #Ahora reflejo si es el caso
    if side=="L":
      clipeado=flipDato(clipeado)

    # Calculo coverage total
    coverage_total = calcular_area_blanca_manual(clipeado)

    # Calculo el coverage de las áreas
    coverage_areas = calcular_area_blanca_regiones_manual(clipeado)

    # Calculo la simetría
    simetria = calcular_simetria(clipeado)

    # Calculo el balance
    balance = calcular_balance(clipeado)

    # Calculo el centro de masa
    cmx, cmy = centro_de_masa(clipeado)

    # Construyo la fila (diccionario con todas las features)
    fila = {
        'STD': std,
        'Kurtosis': kurtosis,
        'Coverage_Total': coverage_total,
        'Coverage_REG1': coverage_areas[0,0],
        'Coverage_REG2': coverage_areas[0,1],
        'Coverage_REG3': coverage_areas[0,2],
        'Coverage_REG4': coverage_areas[1,0],
        'Coverage_REG5': coverage_areas[1,1],
        'Coverage_REG6': coverage_areas[1,2],
        'Simetria': simetria,
        'Balance': balance,
        'CM_X': cmx,
        'CM_Y': cmy
    }

    # Cargo las características y modelo
    elementos = os.listdir(os.path.join("PositionModel"))

    caracteristicasSeleccionadas = None
    model = None

    for elemento in elementos:
        ruta = os.path.join("PositionModel", elemento)
        if elemento.endswith(".txt"):
            with open(ruta, "r", encoding="utf-8") as w:
                for linea in w.readlines():
                    if linea.startswith("["):
                        caracteristicasSeleccionadas = ast.literal_eval(linea)
        elif elemento.endswith(".joblib"):
            model = load(ruta)

    if model is None or caracteristicasSeleccionadas is None:
        raise ValueError("No se encontró el modelo o las características seleccionadas en 'PositionModel'")

    # Construyo X (vector de entrada al modelo)
    X = np.array([fila[feat] for feat in caracteristicasSeleccionadas]).reshape(1, -1)

    # Predicción
    pred = model.predict(X)

    return pred
