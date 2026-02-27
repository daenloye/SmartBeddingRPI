// auth.interceptor.ts
import { HttpInterceptorFn, HttpErrorResponse } from '@angular/common/http';
import { inject } from '@angular/core';
import { catchError, throwError } from 'rxjs';
import { ApiService } from '../services/api.service';

export const authInterceptor: HttpInterceptorFn = (req, next) => {
  const apiService = inject(ApiService);
  const token = apiService.getToken();

  // Clonamos la petición para añadir el header si existe el token
  let authReq = req;
  if (token) {
    authReq = req.clone({
      setHeaders: { Authorization: `Bearer ${token}` }
    });
  }

  return next(authReq).pipe(
    catchError((error: HttpErrorResponse) => {
      if (error.status === 401) {
        // Si llega un 401 de cualquier lado, cerramos sesión
        console.log("Cerrando sesion por 401 con token: ", token);
        apiService.handleUnauthorized();
      }
      return throwError(() => error);
    })
  );
};
