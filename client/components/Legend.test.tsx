import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import Legend from './Legend';

describe('Legend Component Unit Tests', () => {
    it('should display only the toggle button initially', () => {
        render(<Legend />);

        const toggleBtn = screen.getByTitle('Afficher la légende');
        expect(toggleBtn).toBeInTheDocument();
        expect(toggleBtn).toHaveTextContent('?');

        // The legend details should not be in the document
        expect(screen.queryByText('Légende')).not.toBeInTheDocument();
    });

    it('should display the legend items when clicked', () => {
        render(<Legend />);

        const toggleBtn = screen.getByTitle('Afficher la légende');
        
        // Open the legend
        fireEvent.click(toggleBtn);

        expect(toggleBtn).toHaveAttribute('title', 'Fermer la légende');
        expect(screen.getByText('Légende')).toBeInTheDocument();

        // Check if various items from LEGEND_ITEMS are rendered
        expect(screen.getByText('Intersection')).toBeInTheDocument();
        expect(screen.getByText('Habitation')).toBeInTheDocument();
        expect(screen.getByText('Travail')).toBeInTheDocument();
        expect(screen.getByText('Route bidirectionnelle')).toBeInTheDocument();
        expect(screen.getByText('Route à sens unique')).toBeInTheDocument();
    });

    it('should close the legend when clicked a second time', () => {
        render(<Legend />);

        const toggleBtn = screen.getByTitle('Afficher la légende');

        // Open legend
        fireEvent.click(toggleBtn);
        expect(screen.getByText('Légende')).toBeInTheDocument();

        // Close legend
        const closeBtn = screen.getByTitle('Fermer la légende');
        fireEvent.click(closeBtn);
        expect(screen.queryByText('Légende')).not.toBeInTheDocument();
    });
});
