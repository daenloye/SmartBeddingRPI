import { Routes } from '@angular/router';
import { LoginComponent } from './login/login.component';
import { TemplatePanelComponent } from './panel/template-panel/template-panel.component';
import { HomeComponent } from './panel/home/home.component';

export const routes: Routes = [
  {path: '', component: LoginComponent},
  {
    path:'panel',
    component: TemplatePanelComponent,
    children:[
      {path: '', component: HomeComponent},
    ]

  }
];
