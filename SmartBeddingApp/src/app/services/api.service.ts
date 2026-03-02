import { ApiLogin } from './../interfaces/api-login';
import { Injectable, inject } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { Observable, tap } from 'rxjs';
import { ApiResponse } from '../interfaces/api-response';
import { Router } from '@angular/router';
import { environment } from '../../environments/environment.development';

@Injectable({ providedIn: 'root' })
export class ApiService {
  // CAMBIO CLAVE: Usamos la ruta relativa que configuramos en Nginx.
  // Al no poner "http://...", el navegador usará automáticamente el dominio actual.
  private readonly API_URL = environment.apiUrl;

  private http = inject(HttpClient);
  private router = inject(Router);

  verifyToken(): Observable<ApiResponse<any>> {
    return this.http.get<ApiResponse<any>>(`${this.API_URL}/verify`).pipe(
      tap(res => {
        if (!res.result) this.handleUnauthorized();
      })
    );
  }

  login(code: string): Observable<ApiResponse<string>> {
    // La ruta final será: http://smartbedding.local/api/auth
    return this.http.post<ApiResponse<string>>(`${this.API_URL}/auth`, { code }).pipe(
      tap(res => {
        if (res.result && res.data) {
          localStorage.setItem('access_token', res.data);
          this.router.navigate(['/panel']);
        }
      })
    );
  }

  handleUnauthorized() {
    this.logout();
    this.router.navigate(['/']);
  }

  getToken(): string | null {
    return localStorage.getItem('access_token');
  }

  isLoggedIn(): boolean {
    return !!this.getToken();
  }

  logout() {
    localStorage.removeItem('access_token');
  }

  getConnectivity(): Observable<ApiResponse<any>> {
    return this.http.get<ApiResponse<any>>(`${this.API_URL}/connectivity`);
  }

  getStorage(): Observable<ApiResponse<any>> {
    return this.http.get<ApiResponse<any>>(`${this.API_URL}/storage`);
  }

  clearStorage(): Observable<any> {
    return this.http.delete<ApiResponse<any>>(`${this.API_URL}/storage`);
  }
}
