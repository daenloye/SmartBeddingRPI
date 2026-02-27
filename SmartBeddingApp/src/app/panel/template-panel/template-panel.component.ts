import { Component } from '@angular/core';
import { RouterLink, RouterOutlet } from '@angular/router';

@Component({
  selector: 'app-template-panel',
  imports: [RouterOutlet,RouterLink],
  templateUrl: './template-panel.component.html',
  styleUrl: './template-panel.component.css'
})
export class TemplatePanelComponent {

}
