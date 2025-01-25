import pickle

class Entity:
    def __init__(self):
        """Initializes an entity."""

        self.id = 0
        self.manager_id = 0

        self.type = EntityType.NPC

    def serialize(self):
        """Serializes the Entity to a pickle string."""
        return pickle.dumps(self)

    @staticmethod
    def deserialize(data):
        """Deserializes an Entity from a pickle string."""
        return pickle.loads(data)

    def event(self, event, value):
        """Event"""
        pass

    def user_event(self, event, value):
        """User event. Only send to EntityType.PLAYER"""
