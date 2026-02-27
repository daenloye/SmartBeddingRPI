import { ApiLogin } from './../interfaces/api-login';
import { Injectable, inject } from '@angular/core';
import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { Observable, tap } from 'rxjs';
import { ApiResponse } from '../interfaces/api-response';
import { Router } from '@angular/router';

@Injectable({ providedIn: 'root' })
export class ApiService {
  private readonly API_URL = 'http://192.168.0.112:8080';
  private http = inject(HttpClient);
  private router = inject(Router);

// Este método lo llamaremos desde el Guard o manualmente
  verifyToken(): Observable<ApiResponse<any>> {
    return this.http.get<ApiResponse<any>>(`${this.API_URL}/verify`).pipe(
      tap(res => {
        if (!res.result) this.handleUnauthorized();
      })
    );
  }

  login(code: string): Observable<ApiResponse<string>> {
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
}
